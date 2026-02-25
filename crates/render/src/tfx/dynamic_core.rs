use anyhow::Context;
use deimos_data::{
    tag::WideHash,
    tfx::{ExternIndex, SDynamicCore, SSamplerReference},
};
use glam::Vec4;
use itertools::Itertools;
use tiger_pkg::{TagHash, package_manager};

use crate::{
    gpu::command_list::CommandList,
    tfx::expression_vm::{
        self,
        interpreter::InterpreterState,
        opcodes::{Opcode, OpcodeIterator},
    },
};

/// Shared core for dynamic textures/samplers/constants used by scopes and techniques
///
/// # D3D12 Related Notes
/// - Samplers are extracted and filtered from bytecode, and used as static samplers in the pipeline state object.
/// - Dynamic textures are also extracted, and joined together with static textures in the `textures` array.
pub struct DynamicCore {
    shader_visibility: d3d12::ShaderVisibility,

    data: SDynamicCore,

    pub samplers: Vec<d3d12::StaticSamplerDesc>,
    pub textures: Vec<(u32, TextureSource)>,

    initial_constants: Vec<Vec4>,
    cbuffer_size: usize,
}

impl DynamicCore {
    pub fn new(
        data: SDynamicCore,
        shader_visibility: d3d12::ShaderVisibility,
    ) -> anyhow::Result<Self> {
        let mut core = Self {
            shader_visibility,
            data,
            samplers: Vec::new(),
            textures: Vec::new(),
            initial_constants: Vec::new(),
            cbuffer_size: 0,
        };

        let mut sampler_tags = Vec::new();

        extract_textures_and_samplers(
            &core.data.bytecode,
            &core.data.samplers,
            &mut sampler_tags,
            &mut core.textures,
        )?;

        for (slot, tag) in sampler_tags {
            let sampler_entry = package_manager()
                .get_entry(tag)
                .context("missing entry for sampler")?;
            let data = package_manager()
                .read_tag(sampler_entry.reference)
                .context("reading sampler")?;
            let sampler: d3d12::SamplerDesc =
                unsafe { data.as_ptr().cast::<d3d12::SamplerDesc>().read() };
            core.samplers.push(d3d12::StaticSamplerDesc {
                filter: sampler.filter,
                address_u: sampler.address_u,
                address_v: sampler.address_v,
                address_w: sampler.address_w,
                mip_lod_bias: sampler.mip_lod_bias,
                max_anisotropy: sampler.max_anisotropy,
                comparison_func: sampler.comparison_func,
                border_color: d3d12::D3D12_STATIC_BORDER_COLOR_TRANSPARENT_BLACK,
                min_lod: sampler.min_lod,
                max_lod: sampler.max_lod,
                shader_register: slot,
                register_space: 0,
                shader_visibility,
            });
        }

        core.data.bytecode =
            filter_bytecode_assignments(&core.data.bytecode).context("filtering bytecode")?;

        core.initial_constants = if core.data.constant_buffer.is_some() {
            let entry = package_manager()
                .get_entry(core.data.constant_buffer)
                .context("Failed to get cbuffer tag entry")?;

            let data = package_manager().read_tag(entry.reference)?;
            let vec4s = bytemuck::cast_slice(&data);
            vec4s.to_vec()
        } else {
            let vec4s = &core.data.unk30;
            if vec4s.is_empty() {
                vec![]
            } else {
                vec4s.to_vec()
            }
        };
        core.cbuffer_size = core.initial_constants.len();

        Ok(core)
    }

    pub fn cbuffer_slot(&self) -> i32 {
        self.data.constant_buffer_slot
    }

    pub fn prepare(
        &self,
        cmd: &mut CommandList,
    ) -> anyhow::Result<Option<d3d12::GpuVirtualAddress>> {
        if self.data.constant_buffer.is_some() {
            let mut buffer = cmd
                .gpu()
                .frame()
                .upload
                .alloc(self.cbuffer_size)
                .context("allocating cbuffer")?;

            let out = bytemuck::cast_slice_mut(buffer.as_mut_slice());
            out.copy_from_slice(&self.initial_constants);
            let mut interpreter = InterpreterState::new(&self.data.bytecode);

            if let Err(e) = interpreter.evaluate(&self.data.bytecode_constants, out) {
                error!("Failed to evaluate expression bytecode: {:?}", e);

                let bytecode_listing = match expression_vm::disassemble(&self.data.bytecode) {
                    Ok(ops) => ops.into_iter().map(|v| format!("    {v}")).join("\n"),
                    Err(e) => {
                        format!("Failed to disassemble bytecode: {e:?}")
                    }
                };
                debug!("Bytecode:\n{}", bytecode_listing);

                if interpreter.ip < self.data.bytecode.len() {
                    // Patch the bytecode to disable the expression
                    unsafe {
                        self.data
                            .bytecode
                            .as_ptr()
                            .add(interpreter.ip)
                            .cast_mut()
                            .write(expression_vm::opcodes::Opcode::ExtReturn as u8);
                    }
                }
            }

            Ok(Some(buffer.virtual_address()))
        } else {
            Ok(None)
        }
    }
}

pub enum TextureSource {
    Static(WideHash),
    Dynamic {
        extern_index: ExternIndex,
        offset: u32,
    },
}

fn extract_textures_and_samplers(
    bytecode: &[u8],
    static_samplers: &[SSamplerReference],
    samplers: &mut Vec<(u32, TagHash)>,
    textures: &mut Vec<(u32, TextureSource)>,
) -> anyhow::Result<()> {
    let mut last_sampler = None;
    let mut last_texture = None;

    let bytecode = OpcodeIterator::new(bytecode);
    for op in bytecode {
        let (op, ptr) = op?;

        match op {
            Opcode::PushSamplerState => {
                let index = ptr[0];
                let sampler = static_samplers
                    .get(index as usize)
                    .context("Invalid sampler index")?;
                last_sampler = Some(sampler.sampler);
            }
            Opcode::PopSamplerState => {
                let slot = ptr[0] & 0x1F;
                let sampler = last_sampler
                    .take()
                    .context("Sampler state was not pushed")?;
                samplers.push((slot as u32, sampler));
            }
            Opcode::PushExternInputTextureView => {
                let extern_id = ExternIndex::try_from(ptr[0])
                    .ok()
                    .context("Invalid extern index")?;
                let offset = ptr[1] as u32 * 8;
                last_texture = Some(TextureSource::Dynamic {
                    extern_index: extern_id,
                    offset,
                });
            }
            Opcode::PopTextureView => {
                let slot = ptr[0] & 0x1F;
                let texture = last_texture.take().context("Texture view was not pushed")?;
                textures.push((slot as u32, texture));
            }
            _ => {
                anyhow::ensure!(
                    last_sampler.is_none() && last_texture.is_none(),
                    "Malformed bytecode. Sampler/texture was acquired but not assigned"
                );
            }
        }
    }

    Ok(())
}

fn filter_bytecode_assignments(original: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut new = Vec::with_capacity(original.len());

    let mut offset = 0;
    let bytecode = OpcodeIterator::new(original);
    for op in bytecode {
        let (op, _) = op?;

        if matches!(
            op,
            Opcode::PushSamplerState
                | Opcode::PopSamplerState
                | Opcode::PushExternInputTextureView
                | Opcode::PopTextureView
        ) {
            offset += op.size();
            continue;
        }

        new.extend_from_slice(&original[offset..offset + op.size()]);

        offset += op.size();
    }

    Ok(new)
}
