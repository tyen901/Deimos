use std::sync::Arc;

use anyhow::Context;
use d3d12::Format;
use deimos_data::tfx::{STechnique, TechniqueBindMode};
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use crate::gpu::{Gpu, command_list::CommandList, pipeline_cache::PipelineKey};

pub struct Technique {
    gpu: Arc<Gpu>,
    data: STechnique,

    root_signature: d3d12::RootSignature,
}

impl Technique {
    pub fn new(gpu: &Arc<Gpu>, hash: TagHash) -> anyhow::Result<Self> {
        let data: STechnique = package_manager()
            .read_tag_struct(hash)
            .context("Failed to read technique data")?;

        println!("Compatible Scopes: {:?}", data.compatible_scopes);
        println!("Used Scopes: {:?}", data.used_scopes);

        anyhow::bail!("TODO")
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
