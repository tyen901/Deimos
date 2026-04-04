// pub enum View {
//     Shaded(ShadedView),
//     Shadow(ShadowView),
// }

use std::sync::Arc;

use crate::{
    gpu::{Gpu, render_target::RenderTarget},
    tfx::buffers::{Gbuffer, LightBuffer},
    visibility::frustum::Frustum,
};

pub struct ShadedView {
    pub culling_frustum: Frustum,

    pub gbuffer: Gbuffer,
    pub light: LightBuffer,

    pub output: RenderTarget,

    resolution: (u32, u32),
    resolution_scale: f32,
}

impl ShadedView {
    pub fn new(gpu: &Arc<Gpu>, resolution: (u32, u32)) -> anyhow::Result<Self> {
        Ok(Self {
            culling_frustum: Frustum::default(),
            gbuffer: Gbuffer::new(gpu, resolution)?,
            light: LightBuffer::new(gpu, resolution)?,
            output: RenderTarget::new(
                gpu,
                "result",
                d3d12::Format::R8g8b8a8Unorm,
                d3d12::Format::R8g8b8a8Unorm,
                resolution,
            )?,
            resolution,
            resolution_scale: 1.0,
        })
    }

    pub fn resize(&mut self, gpu: &Arc<Gpu>, new_resolution: (u32, u32)) -> anyhow::Result<()> {
        let new_resolution = (
            ((new_resolution.0 as f32 * self.resolution_scale) as u32).max(64),
            ((new_resolution.1 as f32 * self.resolution_scale) as u32).max(64),
        );
        if new_resolution == self.resolution {
            return Ok(());
        }

        *self = Self::new(gpu, new_resolution)?;
        Ok(())
    }

    pub const fn set_resolution_scale(&mut self, scale: f32) {
        self.resolution_scale = scale;
    }

    pub const fn resolution_scale(&self) -> f32 {
        self.resolution_scale
    }

    pub const fn resolution(&self) -> (u32, u32) {
        self.resolution
    }
}

// pub struct ShadowView {}
