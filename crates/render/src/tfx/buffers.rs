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
    pub normal_read: RenderTarget,
    pub rt3: RenderTarget,
    pub rt4: RenderTarget,

    pub depth: DepthBuffer,
    pub depth_read: DepthBuffer,

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
            normal_read: RenderTarget::new(
                gpu,
                "deferred_normal_read",
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
            depth_read: DepthBuffer::new(gpu, resolution).context("allocating depth buffer")?,

            resolution,
        })
    }

    pub fn clear(&self, cmd: &d3d12::GraphicsCommandList) {
        cmd.clear_render_target_view(self.albedo.cpu_handle(), &[0.0, 0.0, 0.0, 0.0]);
        cmd.clear_render_target_view(self.normal.cpu_handle(), &[0.0, 0.0, 0.0, 0.0]);
        cmd.clear_render_target_view(self.normal_read.cpu_handle(), &[0.0, 0.0, 0.0, 0.0]);
        cmd.clear_render_target_view(self.rt3.cpu_handle(), &[0.0, 0.5, 0.0, 0.0]);
        cmd.clear_render_target_view(self.rt4.cpu_handle(), &[0.0, 0.0, 0.0, 0.0]);
        cmd.clear_depth_stencil_view(self.depth.cpu_handle(), d3d12::ClearFlags::DEPTH, 0.0, 0);
        cmd.clear_depth_stencil_view(
            self.depth_read.cpu_handle(),
            d3d12::ClearFlags::DEPTH,
            0.0,
            0,
        );
    }

    pub fn bind(&self, cmd: &mut CommandList) {
        cmd.set_render_targets(
            &[&self.albedo, &self.normal, &self.rt3, &self.rt4],
            Some(&self.depth),
        );
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

pub struct LightBuffer {
    pub light_diffuse: RenderTarget,
    pub light_specular: RenderTarget,
    pub light_specular_ibl: RenderTarget,

    resolution: (u32, u32),
}

impl LightBuffer {
    pub fn new(gpu: &Arc<Gpu>, resolution: (u32, u32)) -> anyhow::Result<Self> {
        Ok(Self {
            light_diffuse: RenderTarget::new(
                gpu,
                "light_diffuse",
                d3d12::Format::R11g11b10Float,
                d3d12::Format::R11g11b10Float,
                resolution,
            )
            .context("allocating render target")?,
            light_specular: RenderTarget::new(
                gpu,
                "light_specular",
                d3d12::Format::R11g11b10Float,
                d3d12::Format::R11g11b10Float,
                resolution,
            )
            .context("allocating render target")?,
            light_specular_ibl: RenderTarget::new(
                gpu,
                "light_specular_ibl",
                d3d12::Format::R11g11b10Float,
                d3d12::Format::R11g11b10Float,
                resolution,
            )
            .context("allocating render target")?,
            resolution,
        })
    }

    pub fn clear(&self, cmd: &d3d12::GraphicsCommandList) {
        cmd.clear_render_target_view(self.light_diffuse.cpu_handle(), &[0.0, 0.0, 0.0, 0.0]);
        cmd.clear_render_target_view(self.light_specular.cpu_handle(), &[0.0, 0.0, 0.0, 0.0]);
        cmd.clear_render_target_view(self.light_specular_ibl.cpu_handle(), &[0.0, 0.0, 0.0, 0.0]);
    }

    pub fn bind_for_lights(&self, cmd: &mut CommandList) {
        cmd.set_render_targets(&[&self.light_diffuse, &self.light_specular], None);

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

    pub fn bind_for_cubemaps(&self, cmd: &mut CommandList) {
        cmd.set_render_targets(&[&self.light_diffuse, &self.light_specular_ibl], None);

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
        self.light_diffuse.transition(cmd, new_state);
        self.light_specular.transition(cmd, new_state);
        self.light_specular_ibl.transition(cmd, new_state);
    }
}
