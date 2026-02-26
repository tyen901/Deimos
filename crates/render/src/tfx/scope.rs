use anyhow::Context;
use deimos_data::tfx::{
    ShaderStage,
    scope::{SScope, SScopeStage},
};
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use crate::{gpu::command_list::CommandList, tfx::dynamic_core::DynamicCore};

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
            stage_vertex: ScopeStage::new(data.stage_vertex.clone(), ShaderStage::Vertex)
                .context("while loading vertex stage")?,
            stage_pixel: ScopeStage::new(data.stage_pixel.clone(), ShaderStage::Pixel)
                .context("while loading pixel stage")?,
            data,
        })
    }

    pub fn bind(&self, cmd: &mut CommandList) {
        for stage in self.all_stages() {
            stage.bind(cmd);
        }
    }

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
    pub fn new(stage: SScopeStage, shader_stage: ShaderStage) -> anyhow::Result<Self> {
        let core = DynamicCore::new(stage.core, shader_stage)?;
        Ok(Self {
            core,
            visibility: shader_stage.shader_visibility(),
        })
    }

    pub fn bind(&self, cmd: &mut CommandList) {
        if let Err(e) = self.core.prepare(cmd) {
            error!("Failed to prepare technique: {}", e);
        }
    }
}
