// pub enum View {
//     Shaded(ShadedView),
//     Shadow(ShadowView),
// }

use std::sync::Arc;

use crate::{
    gpu::{Gpu, render_target::RenderTarget},
    tfx::buffers::Gbuffer,
    visibility::frustum::Frustum,
};

pub struct ShadedView {
    pub culling_frustum: Frustum,

    pub gbuffer: Gbuffer,

    pub result: RenderTarget,

    resolution: (u32, u32),
}

impl ShadedView {
    pub fn new(gpu: &Arc<Gpu>, resolution: (u32, u32)) -> anyhow::Result<Self> {
        Ok(Self {
            culling_frustum: Frustum::default(),
            gbuffer: Gbuffer::new(gpu, resolution)?,
            result: RenderTarget::new(
                gpu,
                "result",
                d3d12::Format::R8g8b8a8Unorm,
                d3d12::Format::R8g8b8a8Unorm,
                resolution,
            )?,
            resolution,
        })
    }

    pub fn resize(&mut self, gpu: &Arc<Gpu>, new_resolution: (u32, u32)) -> anyhow::Result<()> {
        if new_resolution == self.resolution {
            return Ok(());
        }

        *self = Self::new(gpu, new_resolution)?;
        Ok(())
    }
}

// pub struct ShadowView {}
