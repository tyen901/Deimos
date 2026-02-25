use std::sync::Arc;

use anyhow::Context;
use d3d12::{DescriptorRange, Format, RootSignatureBuilder, RootSignatureFlags};
use deimos_data::tfx::{STechnique, STechniqueStage, TechniqueBindMode};
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use crate::{
    gpu::{Gpu, command_list::CommandList, pipeline_cache::PipelineKey},
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
            TechniqueStage::new(data.shader_vertex.clone(), d3d12::ShaderVisibility::Vertex)
                .context("while loading vertex stage")?;

        let mut stage_pixel =
            TechniqueStage::new(data.shader_pixel.clone(), d3d12::ShaderVisibility::Pixel)
                .context("while loading pixel stage")?;

        let mut rsb = RootSignatureBuilder::default()
            .flags(RootSignatureFlags::ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT);

        let mut descriptor_ranges = Vec::new();
        let mut descriptor_offset = 0;
        for stage in [&mut stage_vertex, &mut stage_pixel] {
            for scopes in data.used_scopes.iter() {
                let index = scopes.bits().ilog2();
            }

            for sampler in &stage.core.samplers {
                rsb.add_sampler(sampler.clone());
            }

            let mut local_ranges = vec![];
            for &(texture_slot, _) in &stage.core.textures {
                let range = d3d12::DescriptorRange {
                    range_type: d3d12::DescriptorRangeType::Srv,
                    num_descriptors: 1,
                    base_shader_register: texture_slot,
                    register_space: 0,
                    offset_in_descriptors_from_table_start: descriptor_offset,
                };
                stage.root_texture_slots[texture_slot as usize] = Some(descriptor_offset);
                descriptor_offset += 1;
                local_ranges.push(range);
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
            }
            Err(err) => {
                error!("Failed to create pipeline: {}", err);
            }
        }
    }
}

pub struct TechniqueStage {
    pub core: DynamicCore,
    pub visibility: d3d12::ShaderVisibility,

    pub root_cbuffer_slots: [Option<u8>; 32],
    pub root_texture_slots: [Option<u32>; 32],
}

impl TechniqueStage {
    pub fn new(
        stage: STechniqueStage,
        shader_visibility: d3d12::ShaderVisibility,
    ) -> anyhow::Result<Self> {
        let core = DynamicCore::new(stage.core, shader_visibility)?;
        Ok(Self {
            core,
            visibility: shader_visibility,
            root_cbuffer_slots: [None; 32],
            root_texture_slots: [None; 32],
        })
    }
}
