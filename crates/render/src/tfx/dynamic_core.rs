use anyhow::Context;
use deimos_data::{
    tag::WideHash,
    tfx::{ExternIndex, SDynamicCore, SSamplerReference},
};
use tiger_pkg::{TagHash, package_manager};

use crate::tfx::expression_vm::opcodes::{Opcode, OpcodeIterator};

/// Shared core for dynamic textures/samplers/constants used by scopes and techniques
///
/// # D3D12 Related Notes
/// - Samplers are extracted and filtered from bytecode, and used as static samplers in the pipeline state object.
/// - Dynamic textures are also extracted, and joined together with static textures in the `textures` array.
pub struct DynamicCore {
    data: SDynamicCore,

    pub samplers: Vec<(u32, d3d12::SamplerDesc)>,
    pub textures: Vec<(u32, TextureSource)>,
}

impl DynamicCore {
    pub fn new(data: SDynamicCore) -> anyhow::Result<Self> {
        let mut core = Self {
            data,
            samplers: Vec::new(),
            textures: Vec::new(),
        };

        let mut sampler_tags = Vec::new();

        extract_textures_and_samplers(
            &core.data.bytecode,
            &core.data.samplers,
            &mut sampler_tags,
            &mut core.textures,
        )?;

        for (slot, tag) in sampler_tags {
            let data = package_manager().read_tag(tag).context("reading sampler")?;
            let sampler: d3d12::SamplerDesc =
                unsafe { data.as_ptr().cast::<d3d12::SamplerDesc>().read() };
            core.samplers.push((slot, sampler));
        }

        core.data.bytecode =
            filter_bytecode_assignments(&core.data.bytecode).context("filtering bytecode")?;

        Ok(core)
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
