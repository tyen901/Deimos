use std::sync::Arc;

use anyhow::Context;
use deimos_data::tfx::{
    ShaderStage,
    scope::{SScope, SScopeStage},
};
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use crate::{
    asset::AssetManager,
    gpu::command_list::CommandList,
    tfx::dynamic_core::{DynamicCore, DynamicCoreResources},
};

pub struct Scope {
    _data: SScope,
    // root_signature: d3d12::RootSignature,
    stage_vertex: ScopeStage,
    stage_pixel: ScopeStage,
}

impl Scope {
    pub fn load(asset_manager: &Arc<AssetManager>, hash: TagHash) -> anyhow::Result<Self> {
        let data: SScope = package_manager()
            .read_tag_struct(hash)
            .context("Failed to read scope data")?;

        Ok(Self {
            stage_vertex: ScopeStage::new(
                asset_manager,
                data.stage_vertex.clone(),
                ShaderStage::Vertex,
            )
            .context("while loading vertex stage")?,
            stage_pixel: ScopeStage::new(
                asset_manager,
                data.stage_pixel.clone(),
                ShaderStage::Pixel,
            )
            .context("while loading pixel stage")?,
            _data: data,
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
    pub fn new(
        asset_manager: &Arc<AssetManager>,
        stage: SScopeStage,
        shader_stage: ShaderStage,
    ) -> anyhow::Result<Self> {
        let core = DynamicCore::new(asset_manager, stage.core, shader_stage)?;
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

pub struct ScopeSamplers {
    stage_vertex: ScopeStageSamplers,
    stage_pixel: ScopeStageSamplers,
}

impl ScopeSamplers {
    pub fn load(hash: TagHash) -> anyhow::Result<Self> {
        let data: SScope = package_manager()
            .read_tag_struct(hash)
            .context("Failed to read scope data")?;

        Ok(Self {
            stage_vertex: ScopeStageSamplers::new(data.stage_vertex.clone(), ShaderStage::Vertex)
                .context("while loading vertex stage")?,
            stage_pixel: ScopeStageSamplers::new(data.stage_pixel.clone(), ShaderStage::Pixel)
                .context("while loading pixel stage")?,
        })
    }

    pub fn all_stages(&self) -> [&ScopeStageSamplers; 2] {
        [&self.stage_vertex, &self.stage_pixel]
    }

    pub fn stage_by_visibility(
        &self,
        visibility: d3d12::ShaderVisibility,
    ) -> Option<&ScopeStageSamplers> {
        self.all_stages()
            .into_iter()
            .find(|stage| stage.visibility == visibility)
    }
}

pub struct ScopeStageSamplers {
    pub core: DynamicCoreResources,
    visibility: d3d12::ShaderVisibility,
}

impl ScopeStageSamplers {
    pub fn new(stage: SScopeStage, shader_stage: ShaderStage) -> anyhow::Result<Self> {
        let core = DynamicCoreResources::extract(&stage.core, shader_stage)?;
        Ok(Self {
            core,
            visibility: shader_stage.shader_visibility(),
        })
    }
}
