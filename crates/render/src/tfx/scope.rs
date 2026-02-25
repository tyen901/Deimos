use std::sync::Arc;

use anyhow::Context;
use d3d12::Format;
use deimos_data::tfx::{
    STechnique, STechniqueStage, TechniqueBindMode,
    scope::{SScope, SScopeStage},
};
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use crate::{
    gpu::{Gpu, command_list::CommandList, pipeline_cache::PipelineKey},
    tfx::dynamic_core::DynamicCore,
};

pub struct Scope {
    data: SScope,
    // root_signature: d3d12::RootSignature,
    stage_vertex: ScopeStage,
    stage_pixel: ScopeStage,
}

impl Scope {
    pub fn load(hash: TagHash) -> anyhow::Result<Self> {
        let data: SScope = package_manager()
            .read_tag_struct(hash)
            .context("Failed to read scope data")?;

        Ok(Self {
            stage_vertex: ScopeStage::new(
                data.stage_vertex.clone(),
                d3d12::ShaderVisibility::Vertex,
            )
            .context("while loading vertex stage")?,
            stage_pixel: ScopeStage::new(data.stage_pixel.clone(), d3d12::ShaderVisibility::Pixel)
                .context("while loading pixel stage")?,
            data,
        })
    }

    // pub fn bind(&self, cmd: &mut CommandList) {
    //     if self.data.bind_mode != TechniqueBindMode::VertexPixel {
    //         error!("{:?} bind mode not implemented", self.data.bind_mode);
    //         return;
    //     }

    //     let fixed_function_state = cmd
    //         .state
    //         .select(&self.data.states)
    //         .select(&cmd.state_override);

    //     let pipeline_key = PipelineKey {
    //         vertex_shader: self.data.shader_vertex.shader,
    //         pixel_shader: self.data.shader_pixel.shader,
    //         fixed_function_state,
    //         input_layout: cmd.get_input_layout() as u8,
    //     };

    //     match self.gpu.pipeline_cache.lock().get_or_create(
    //         pipeline_key,
    //         todo!(),
    //         // &self.root_signature,
    //         &[
    //             Format::R8g8b8a8Unorm,
    //             Format::R11g11b10Float,
    //             Format::R8g8b8a8Unorm,
    //             Format::R8g8b8a8Unorm,
    //         ],
    //         Some(Format::D32FloatS8x24Uint),
    //     ) {
    //         Ok(pipeline) => {
    //             cmd.set_pipeline_state(&pipeline.pso);
    //         }
    //         Err(err) => {
    //             error!("Failed to create pipeline: {}", err);
    //         }
    //     }
    // }

    pub fn all_stages(&self) -> [&ScopeStage; 2] {
        [&self.stage_vertex, &self.stage_pixel]
    }

    pub fn stage_by_visibility(&self, visibility: d3d12::ShaderVisibility) -> Option<&ScopeStage> {
        self.all_stages()
            .into_iter()
            .find(|stage| stage.visibility == visibility)
    }
}

pub struct ScopeStage {
    pub core: DynamicCore,
    pub visibility: d3d12::ShaderVisibility,
}

impl ScopeStage {
    pub fn new(
        stage: SScopeStage,
        shader_visibility: d3d12::ShaderVisibility,
    ) -> anyhow::Result<Self> {
        let core = DynamicCore::new(stage.core, shader_visibility)?;
        Ok(Self {
            core,
            visibility: shader_visibility,
        })
    }
}
