use std::sync::Arc;

use anyhow::Context;
use chroma_dbg::ChromaDebug;
use d3d12::{
    DescriptorRange, Format, RootSignatureBuilder, RootSignatureFlags, ext::PsvResourceBinding,
};
use deimos_data::tfx::{STechnique, STechniqueStage, ShaderStage, TechniqueBindMode};
use smallvec::SmallVec;
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use crate::{
    gpu::{Gpu, command_list::CommandList, pipeline_cache::PipelineKey},
    renderer::globals::get_scope,
    tfx::dynamic_core::DynamicCore,
};

pub struct Technique {
    gpu: Arc<Gpu>,
    data: STechnique,
    root_signature: d3d12::RootSignature,
    stage_vertex: TechniqueStage,
    stage_pixel: TechniqueStage,
}

impl Technique {
    pub fn load(gpu: &Arc<Gpu>, hash: TagHash) -> anyhow::Result<Self> {
        let data: STechnique = package_manager()
            .read_tag_struct(hash)
            .context("Failed to read technique data")?;

        let mut stage_vertex =
            TechniqueStage::new(gpu, data.shader_vertex.clone(), ShaderStage::Vertex)
                .context("while loading vertex stage")?;

        let mut stage_pixel =
            TechniqueStage::new(gpu, data.shader_pixel.clone(), ShaderStage::Pixel)
                .context("while loading pixel stage")?;

        let mut rsb = RootSignatureBuilder::default()
            .flags(RootSignatureFlags::ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT);

        let mut descriptor_ranges = Vec::new();
        let mut descriptor_offset = 0;
        for stage in [&mut stage_vertex, &mut stage_pixel] {
            for scopes in data.used_scopes.iter() {
                let index = scopes.bits().ilog2();
                let scope = get_scope(index);
                if let Some(scope_stage) = scope.stage_by_visibility(stage.visibility) {
                    for sampler in &scope_stage.core.samplers {
                        rsb.add_sampler(sampler.clone());
                    }
                }
            }

            for sampler in &stage.core.samplers {
                rsb.add_sampler(sampler.clone());
            }

            let mut local_ranges = vec![];
            for resource in &stage.resources {
                match resource.res_type {
                    d3d12::ext::PsvResourceType::SRVTyped
                    | d3d12::ext::PsvResourceType::SRVRaw
                    | d3d12::ext::PsvResourceType::SRVStructured => {
                        let range = d3d12::DescriptorRange {
                            range_type: d3d12::DescriptorRangeType::Srv,
                            num_descriptors: 1,
                            base_shader_register: resource.lower_bound,
                            register_space: 0,
                            offset_in_descriptors_from_table_start: descriptor_offset,
                        };
                        debug_assert!(descriptor_offset <= 0xff);
                        debug_assert!(resource.lower_bound <= 0xff);
                        stage.root_texture_slots.push(TechniqueResourceSlot {
                            descriptor_offset: descriptor_offset as u8,
                            register: resource.lower_bound as u8,
                        });
                        descriptor_offset += 1;
                        local_ranges.push(range);
                    }
                    d3d12::ext::PsvResourceType::CBV => {
                        let rs_index = rsb.add_param(
                            d3d12::RootParameter::CbvDescriptor {
                                shader_register: resource.lower_bound,
                                register_space: 0,
                            },
                            stage.visibility,
                        );
                        debug_assert!(rs_index <= 0xff);
                        debug_assert!(resource.lower_bound <= 0xff);
                        stage.root_cbuffer_slots.push(TechniqueCbufferSlot {
                            register: resource.lower_bound as u8,
                            rs_slot: rs_index as u8,
                        });
                    }
                    _ => {}
                }
            }
            if !local_ranges.is_empty() {
                descriptor_ranges.push((local_ranges, stage.visibility));
            }
        }
        for (ranges, visibility) in descriptor_ranges.iter() {
            rsb.add_param(
                d3d12::RootParameter::DescriptorTable(ranges.as_slice()),
                *visibility,
            );
        }

        let root_signature_raw = rsb.serialize()?;

        let root_signature = gpu.create_root_signature(&root_signature_raw)?;

        Ok(Self {
            gpu: gpu.clone(),
            root_signature,
            stage_vertex,
            stage_pixel,
            data,
        })
    }

    pub fn bind(&self, cmd: &mut CommandList) {
        if self.data.bind_mode != TechniqueBindMode::VertexPixel {
            error!("{:?} bind mode not implemented", self.data.bind_mode);
            return;
        }

        let fixed_function_state = cmd
            .state
            .select(&self.data.states)
            .select(&cmd.state_override);

        let pipeline_key = PipelineKey {
            vertex_shader: self.data.shader_vertex.shader,
            pixel_shader: self.data.shader_pixel.shader,
            fixed_function_state,
            input_layout: cmd.get_input_layout() as u8,
        };

        match self.gpu.pipeline_cache.lock().get_or_create(
            pipeline_key,
            &self.root_signature,
            &[
                Format::R8g8b8a8Unorm,
                Format::R11g11b10Float,
                Format::R8g8b8a8Unorm,
                Format::R8g8b8a8Unorm,
            ],
            Some(Format::D32FloatS8x24Uint),
        ) {
            Ok(pipeline) => {
                cmd.set_pipeline_state(&pipeline.pso);
                cmd.set_root_signature(&self.root_signature);
            }
            Err(err) => {
                error!("Failed to create pipeline: {}", err);
                return;
            }
        }

        for stage in [&self.stage_vertex, &self.stage_pixel] {
            stage.bind(cmd);
        }
    }
}

pub struct TechniqueStage {
    pub core: DynamicCore,
    pub visibility: d3d12::ShaderVisibility,

    pub root_cbuffer_slots: SmallVec<[TechniqueCbufferSlot; 16]>,
    pub root_texture_slots: SmallVec<[TechniqueResourceSlot; 16]>,

    pub resources: Vec<PsvResourceBinding>,
}

pub struct TechniqueCbufferSlot {
    pub register: u8,
    pub rs_slot: u8,
}

pub struct TechniqueResourceSlot {
    pub register: u8,
    pub descriptor_offset: u8,
}

impl TechniqueStage {
    pub fn new(
        gpu: &Arc<Gpu>,
        stage: STechniqueStage,
        shader_stage: ShaderStage,
    ) -> anyhow::Result<Self> {
        let mut resources = Vec::new();
        if let Ok(bytecode) = gpu.pipeline_cache.lock().get_or_load_bytecode(stage.shader) {
            if let Some(parsed_resources) = d3d12::ext::parse_psv0_resources(&bytecode) {
                resources = parsed_resources;
            }
        }

        let core = DynamicCore::new(stage.core, shader_stage)?;
        Ok(Self {
            core,
            visibility: shader_stage.shader_visibility(),
            root_cbuffer_slots: SmallVec::new(),
            root_texture_slots: SmallVec::new(),
            resources,
        })
    }

    pub fn bind(&self, cmd: &mut CommandList) {
        if let Err(e) = self.core.prepare(cmd) {
            error!("Failed to prepare technique: {}", e);
            return;
        }

        for slot in &self.root_cbuffer_slots {
            if let Some(va) =
                cmd.get_shader_constant_buffer_view(self.core.stage, slot.register as u32)
            {
                cmd.set_graphics_root_constant_buffer_view(slot.rs_slot as u32, va);
            } else {
                // error!(
                //     "Missing constant buffer view for register {}",
                //     slot.register
                // );
                cmd.set_graphics_root_constant_buffer_view(
                    slot.rs_slot as u32,
                    d3d12::GpuVirtualAddress::NULL,
                );
            }
        }
        // for slot in &self.root_texture_slots {
        //     cmd.set_root_texture(slot.register, slot.descriptor_offset);
        // }
    }
}
