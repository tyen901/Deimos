use std::sync::Arc;

use anyhow::Context;

use crate::gpu::{
    Gpu,
    command_list::CommandList,
    render_target::{DepthBuffer, RenderTarget},
};

pub struct Gbuffer {
    pub albedo: RenderTarget,
    pub normal: RenderTarget,
    pub rt3: RenderTarget,
    pub rt4: RenderTarget,

    pub depth: DepthBuffer,

    resolution: (u32, u32),
}

impl Gbuffer {
    pub fn new(gpu: &Arc<Gpu>, resolution: (u32, u32)) -> anyhow::Result<Self> {
        Ok(Self {
            albedo: RenderTarget::new(
                gpu,
                "deferred_albedo",
                d3d12::Format::R8g8b8a8Unorm,
                d3d12::Format::R8g8b8a8Unorm,
                resolution,
            )
            .context("allocating render target")?,
            normal: RenderTarget::new(
                gpu,
                "deferred_normal",
                d3d12::Format::R10g10b10a2Typeless,
                d3d12::Format::R10g10b10a2Unorm,
                resolution,
            )
            .context("allocating render target")?,
            rt3: RenderTarget::new(
                gpu,
                "deferred_rt3",
                d3d12::Format::R8g8b8a8Unorm,
                d3d12::Format::R8g8b8a8Unorm,
                resolution,
            )
            .context("allocating render target")?,
            rt4: RenderTarget::new(
                gpu,
                "deferred_horizontal_coordinates",
                d3d12::Format::R32g32Float,
                d3d12::Format::R32g32Float,
                resolution,
            )
            .context("allocating render target")?,
            depth: DepthBuffer::new(gpu, resolution).context("allocating depth buffer")?,

            resolution,
        })
    }

    pub fn clear(&self, cmd: &d3d12::GraphicsCommandList) {
        cmd.clear_render_target_view(self.albedo.cpu_handle(), &[0.0, 0.0, 0.0, 0.0]);
        cmd.clear_render_target_view(self.normal.cpu_handle(), &[0.0, 0.0, 0.0, 0.0]);
        cmd.clear_render_target_view(self.rt3.cpu_handle(), &[0.0, 0.5, 0.0, 0.0]);
        cmd.clear_render_target_view(self.rt4.cpu_handle(), &[0.0, 0.0, 0.0, 0.0]);
        cmd.clear_depth_stencil_view(self.depth.cpu_handle(), d3d12::ClearFlags::DEPTH, 0.0, 0);
    }

    pub fn bind(&self, cmd: &mut CommandList) {
        cmd.om_set_render_targets(
            &[
                self.albedo.cpu_handle(),
                self.normal.cpu_handle(),
                self.rt3.cpu_handle(),
                self.rt4.cpu_handle(),
            ],
            Some(self.depth.cpu_handle()),
        );

        cmd.set_viewports(&[d3d12::Viewport::builder()
            .width(self.resolution.0 as f32)
            .height(self.resolution.1 as f32)
            .build()]);
        cmd.set_scissor_rects(&[d3d12::Rect::builder()
            .right(self.resolution.0 as i32)
            .bottom(self.resolution.1 as i32)
            .top(0)
            .left(0)
            .build()]);
    }

    pub fn transition(
        &mut self,
        cmd: &d3d12::GraphicsCommandList,
        new_state: d3d12::ResourceStates,
    ) {
        self.albedo.transition(cmd, new_state);
        self.normal.transition(cmd, new_state);
        self.rt3.transition(cmd, new_state);
        self.rt4.transition(cmd, new_state);
    }
}
