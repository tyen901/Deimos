//! This module provides view and frame packet management for the renderer.
//!
//! You can think of it as a "sub-renderer" that can exist alongside other sub-renderers and the main renderer without state mucking.

use std::sync::Arc;

use deimos_core::job::SCHEDULER;
use deimos_data::{
    hash::fnv1,
    tfx::{FixedFunctionState, RenderStage, features::dynamic::RenderStageSubscription},
};
use glam::{Vec3, Vec4};

use crate::{
    ecs::{node_visibility_test, populate_submit_nodes},
    gpu::{
        Gpu,
        command_list::{CommandList, DepthMode},
        stream::{FrameCommandStream, ParallelCommandBlock},
    },
    renderer::{
        Renderer,
        packet::{FramePacket, ViewPacket},
    },
    tfx::view::ShadedView,
    util::range::RangeChunks,
    visibility::ViewVisibility,
};

pub struct SceneRenderer {
    pub parent: Arc<Renderer>,

    pub frame_packet: FramePacket,

    pub main_view: ShadedView,

    pub global_channels: [Vec4; 256],
}

impl SceneRenderer {
    pub fn new(parent: Arc<Renderer>) -> anyhow::Result<Self> {
        let mut r = Self {
            frame_packet: FramePacket::default(),
            main_view: ShadedView::new(&parent.gpu, (1920, 1080))?,
            global_channels: parent.globals.channels.default_values(),
            parent,
        };

        r.set_global_channel_by_name("cubemap_sky_intensity", Vec4::splat(0.4));
        r.set_global_channel_by_name("global_cubemap_intensity", Vec4::splat(0.4));

        r.set_global_channel_by_name("global_ambient_intensity", Vec4::splat(0.0));
        r.set_global_channel_by_name("sun_direct_intensity", Vec4::splat(1.3));
        r.set_global_channel_by_id(0xE16B6B6B, Vec4::splat(1000000.0));
        r.set_global_channel_by_id(0x462A7037, Vec4::splat(0.0)); // >0 causes moss to glow?
        r.set_global_channel_by_id(0xCF70AC7C, Vec4::splat(0.0)); // something about transmission
        r.set_global_channel_by_id(0x07806276, Vec4::splat(0.8)); // fixes missing lights, but turning it up also dims transparent objects

        r.set_global_channel_by_id(0x2C538179, Vec4::splat(0.1)); // dims some very bright sky objects
        r.set_global_channel_by_id(0x2E538443, Vec4::splat(1.0)); // fixes marathon/planet rings not appearing

        r.set_global_channel_by_id(0xCF70AC7C, Vec4::splat(1.0)); // something about reflectivity
        r.set_global_channel_by_id(0x471E28F2, Vec4::splat(1.0)); // something about reflectivity

        Ok(r)
    }

    pub fn render(
        &mut self,
        cmd: &mut CommandList,
        visibility: &ViewVisibility,
        debug_pipeline: Option<DebugPipeline>,
        draw_sun_shadows: bool,
    ) {
        self.parent.externs.reset_global_channel_frequencies();
        let gpu = cmd.gpu().clone();
        let stream = &gpu.frame().stream;

        self.parent.globals.scopes.frame.bind(cmd);
        self.parent.globals.scopes.chunk_model.bind(cmd);

        {
            let _scope = self.parent.gpu.profiler_scope(stream, "prepare_per_frame");

            let frame_packet = &self.frame_packet;
            let render_objects = self.parent.objects.read();

            for frame_node in &frame_packet.per_frame_nodes {
                let obj = &render_objects[frame_node.object];
                obj.renderer.prepare_per_frame(cmd, frame_node);
            }
        }

        if draw_sun_shadows && matches!(debug_pipeline, None | Some(DebugPipeline::LookDev)) {
            self.render_shadow_views(&gpu, cmd, stream);
        } else {
            for view in &self.main_view.shadow_views {
                cmd.clear_depth_stencil_view(
                    view.depth.cpu_handle(),
                    d3d12::ClearFlags::DEPTH,
                    1.0,
                    0,
                );
            }
        }

        self.render_main_view(&gpu, cmd, stream, visibility, debug_pipeline);
    }

    fn render_main_view(
        &mut self,
        gpu: &Arc<Gpu>,
        cmd: &mut CommandList,
        stream: &FrameCommandStream,
        visibility: &ViewVisibility,

        debug_pipeline: Option<DebugPipeline>,
    ) {
        let _scope = gpu.profiler_scope(stream, "render_main_view");

        {
            let ext = self.parent.externs.get_mut();
            ext.view.world_to_camera = self.main_view.world_to_camera;
            ext.view.camera_to_projective = self.main_view.camera_to_projective;
            ext.view.derive_matrices(self.main_view.output.resolution());
        }
        self.parent.globals.scopes.view.bind(cmd);

        let use_hdri = debug_pipeline == Some(DebugPipeline::LookDev);

        self.main_view
            .gbuffer
            .transition(cmd, d3d12::ResourceStates::RENDER_TARGET);
        self.main_view
            .gbuffer
            .depth
            .transition(cmd, d3d12::ResourceStates::DEPTH_WRITE);
        self.main_view.gbuffer.clear(cmd);
        self.main_view.gbuffer.bind(cmd);

        // s_extract_render_objects(&self.world, &self.parent);

        // for (_entity, render_object) in self.world.query::<&DynamicRenderObject>().iter() {
        //     self.parent.objects.read()[render_object.handle]
        //         .renderer
        //         .submit(cmd, RenderStage::GenerateGbuffer);
        // }

        {
            let _scope = gpu.profiler_scope(stream, "node_visibility_test");
            node_visibility_test(self, visibility, ShadedView::MAIN_VIEW_ID);
        }

        {
            let _scope = gpu.profiler_scope(stream, "populate_submit_nodes");
            populate_submit_nodes(self, visibility, ShadedView::MAIN_VIEW_ID);
        }

        {
            let _scope = self.parent.gpu.profiler_scope(stream, "prepare_per_view");

            let frame_packet = &self.frame_packet;
            let render_objects = self.parent.objects.read();
            let view = &self.frame_packet.views[ShadedView::MAIN_VIEW_ID];
            for view_node in &view.view_nodes {
                let frame_node = &frame_packet.per_frame_nodes[view_node.frame_node];
                let obj = &render_objects[frame_node.object];
                obj.renderer.prepare_per_view(cmd, frame_node, view_node);
            }
        }

        {
            let view = &self.frame_packet.views[ShadedView::MAIN_VIEW_ID];
            {
                let _scope = self
                    .parent
                    .gpu
                    .profiler_scope(stream, "submit_gbuffer_generation");

                cmd.set_ffstate(FixedFunctionState::new(Some(0), Some(2), Some(2), Some(0)));
                cmd.flush_states();
                self.submit_stage(cmd, stream, view, RenderStage::GenerateGbuffer);
            }

            // Copy normals/depth buffer
            {
                let view = &mut self.main_view;

                // Copy normals
                view.gbuffer
                    .normal
                    .transition(cmd, d3d12::ResourceStates::COPY_SOURCE);
                view.gbuffer
                    .normal_read
                    .transition(cmd, d3d12::ResourceStates::COPY_DEST);

                cmd.copy_resource(
                    view.gbuffer.normal.resource.resource(),
                    view.gbuffer.normal_read.resource.resource(),
                );
                view.gbuffer
                    .normal_read
                    .transition(cmd, d3d12::ResourceStates::PIXEL_SHADER_RESOURCE);
                view.gbuffer
                    .normal
                    .transition(cmd, d3d12::ResourceStates::RENDER_TARGET);

                // Copy depth
                view.gbuffer
                    .depth
                    .transition(cmd, d3d12::ResourceStates::COPY_SOURCE);
                view.gbuffer
                    .depth_read
                    .transition(cmd, d3d12::ResourceStates::COPY_DEST);

                cmd.copy_resource(
                    view.gbuffer.depth.resource.resource(),
                    view.gbuffer.depth_read.resource.resource(),
                );

                view.gbuffer
                    .depth_read
                    .transition(cmd, d3d12::ResourceStates::PIXEL_SHADER_RESOURCE);
                view.gbuffer
                    .depth
                    .transition(cmd, d3d12::ResourceStates::DEPTH_READ);
            }

            {
                let _scope = self.parent.gpu.profiler_scope(stream, "submit_decals");
                self.parent.globals.scopes.decal.bind(cmd);
                cmd.set_ffstate(FixedFunctionState::new(Some(8), Some(15), Some(2), Some(1)));
                cmd.flush_states();
                self.submit_stage_serial(cmd, stream, view, RenderStage::Decals);
            }

            {
                let _scope = self
                    .parent
                    .gpu
                    .profiler_scope(stream, "submit_decals_additive");
                cmd.set_ffstate(FixedFunctionState::new(Some(8), Some(15), Some(2), Some(1)));
                cmd.flush_states();
                self.submit_stage_serial(cmd, stream, view, RenderStage::DecalsAdditive);
            }

            self.main_view
                .gbuffer
                .depth
                .transition(cmd, d3d12::ResourceStates::DEPTH_READ);
            self.main_view
                .gbuffer
                .transition(cmd, d3d12::ResourceStates::PIXEL_SHADER_RESOURCE);
            self.main_view
                .light
                .transition(cmd, d3d12::ResourceStates::RENDER_TARGET);

            {
                let _scope = self
                    .parent
                    .gpu
                    .profiler_scope(stream, "submit_lighting_apply");
                self.main_view.light.clear(cmd);

                if use_hdri {
                    self.main_view.light.bind_for_cubemaps(cmd);
                    let sv = &self.main_view.shadow_views;
                    self.parent.apply_hdri_light(
                        cmd,
                        &[
                            (sv[0].world_to_projective(), sv[0].depth.srv()),
                            (sv[1].world_to_projective(), sv[1].depth.srv()),
                            (sv[2].world_to_projective(), sv[2].depth.srv()),
                            (sv[3].world_to_projective(), sv[3].depth.srv()),
                        ],
                    );
                }

                self.main_view.light.bind_for_lights(cmd);
                cmd.set_ffstate(FixedFunctionState::new(Some(2), None, Some(2), Some(2)));
                cmd.flush_states();
                self.submit_stage(cmd, stream, view, RenderStage::LightingApply);
            }

            self.main_view
                .light
                .transition(cmd, d3d12::ResourceStates::PIXEL_SHADER_RESOURCE);
            self.main_view
                .output
                .transition(cmd, d3d12::ResourceStates::RENDER_TARGET);
            cmd.clear_render_target_view(self.main_view.output.cpu_handle(), &[0.0, 0.0, 0.0, 1.0]);

            cmd.set_ffstate(FixedFunctionState::new(Some(8), Some(15), Some(2), Some(1)));
            if let Some(debug_pipeline) = debug_pipeline {
                cmd.set_render_targets(&[&self.main_view.output], None);

                let p = &self.parent.globals.pipelines;
                let technique = match debug_pipeline {
                    DebugPipeline::LookDev => &p.deferred_shading,
                    DebugPipeline::GlobalLightingShading => &p.global_lighting_and_shading,
                    DebugPipeline::DeferredShading => &p.deferred_shading,
                    DebugPipeline::DeferredShadingNoAtm => &p.deferred_shading_no_atm,
                    DebugPipeline::Albedo => &p.debug_source_color,
                    DebugPipeline::Smoothness => &p.debug_specular_smoothness,
                    DebugPipeline::Metalness => &p.debug_metalness,
                    DebugPipeline::AmbientOcclusion => &p.debug_ambient_occlusion,
                    DebugPipeline::Emission => &p.debug_emissive,
                    DebugPipeline::EmissionIntensity => &p.debug_emissive_intensity,
                    DebugPipeline::Transmission => &p.debug_transmission,
                    DebugPipeline::Overcoat => &p.debug_colored_overcoat_id,
                    DebugPipeline::DepthEdges => &p.debug_depth_edges,
                    DebugPipeline::WorldNormal => &p.debug_world_normal,
                    DebugPipeline::LightDiffuse => &p.debug_diffuse_light,
                    DebugPipeline::LightSpecular => &p.debug_specular_light,

                    DebugPipeline::Overdraw => &p.global_lighting_and_shading,
                };

                self.parent
                    .execute_global_pipeline(cmd, technique, &format!("{debug_pipeline:?}"));

                {
                    self.main_view
                        .shaded_read
                        .transition(cmd, d3d12::ResourceStates::COPY_SOURCE);
                    self.main_view
                        .output
                        .transition(cmd, d3d12::ResourceStates::COPY_DEST);

                    cmd.copy_resource(
                        self.main_view.output.resource.resource(),
                        self.main_view.shaded_read.resource.resource(),
                    );
                }
            } else {
                self.main_view
                    .gbuffer
                    .albedo
                    .transition(cmd, d3d12::ResourceStates::COPY_SOURCE);
                self.main_view
                    .output
                    .transition(cmd, d3d12::ResourceStates::COPY_DEST);

                cmd.copy_resource(
                    self.main_view.gbuffer.albedo.resource.resource(),
                    self.main_view.output.resource.resource(),
                );
            }

            self.main_view
                .output
                .transition(cmd, d3d12::ResourceStates::RENDER_TARGET);

            if use_hdri {
                cmd.set_render_targets(&[&self.main_view.output], None);
                self.parent.draw_hdri_background(cmd);
            }

            {
                cmd.set_render_targets(
                    &[&self.main_view.output],
                    Some(&self.main_view.gbuffer.depth),
                );
                self.parent.globals.scopes.transparent.bind(cmd);
                self.parent.globals.scopes.transparent_advanced.bind(cmd);
                let _scope = self
                    .parent
                    .gpu
                    .profiler_scope(stream, "submit_transparents");
                cmd.set_ffstate(FixedFunctionState::new(Some(8), Some(15), Some(2), Some(1)));
                self.submit_stage_serial(cmd, stream, view, RenderStage::Transparents);

                self.parent.immediate.draw_shapes(cmd);
            }
        }
    }

    fn render_shadow_views(
        &mut self,
        gpu: &Arc<Gpu>,
        cmd: &mut CommandList,
        stream: &FrameCommandStream,
    ) {
        cmd.set_depth_mode(DepthMode::Forward);

        let num_shadow_views = self.main_view.shadow_views.len();
        for shadow_view_index in 0..num_shadow_views {
            let _scope = gpu.profiler_scope(stream, "render_shadow_view");

            let (visibility, view_id) = {
                let shadow_view = &self.main_view.shadow_views[shadow_view_index];

                let ext = self.parent.externs.get_mut();
                ext.view.world_to_camera = shadow_view.world_to_camera;
                ext.view.camera_to_projective = shadow_view.camera_to_projective;
                ext.view.derive_matrices(shadow_view.depth.resolution());

                (
                    ViewVisibility {
                        enabled: true,
                        position: Vec3::ZERO,
                        far_plane: 500.0,
                        culling_frustum: shadow_view.frustum.clone(),
                        world_to_projective: ext.view.world_to_projective,
                        render_stages: RenderStageSubscription::SHADOW_VIEW,
                        occlusion_buffer: None,
                        visible_cluster_bounds: None,
                    },
                    shadow_view.id,
                )
            };

            self.parent.globals.scopes.view.bind(cmd);

            {
                let _scope = gpu.profiler_scope(stream, "node_visibility_test");
                node_visibility_test(self, &visibility, view_id);
            }

            {
                let _scope = gpu.profiler_scope(stream, "populate_submit_nodes");
                populate_submit_nodes(self, &visibility, view_id);
            }

            let shadow_view = &mut self.main_view.shadow_views[shadow_view_index];
            let view = &self.frame_packet.views[shadow_view.id];
            {
                let _scope = gpu.profiler_scope(stream, "prepare_per_view");

                let render_objects = self.parent.objects.read();
                for view_node in &view.view_nodes {
                    let frame_node = &self.frame_packet.per_frame_nodes[view_node.frame_node];
                    let obj = &render_objects[frame_node.object];
                    obj.renderer.prepare_per_view(cmd, frame_node, view_node);
                }
            }

            {
                let _scope = self
                    .parent
                    .gpu
                    .profiler_scope(stream, "submit_shadow_generate");

                shadow_view
                    .depth
                    .transition(cmd, d3d12::ResourceStates::DEPTH_WRITE);
                cmd.clear_depth_stencil_view(
                    shadow_view.depth.cpu_handle(),
                    d3d12::ClearFlags::DEPTH,
                    1.0,
                    0,
                );

                cmd.set_render_targets(&[], Some(&shadow_view.depth));

                cmd.set_ffstate(FixedFunctionState::new(Some(0), Some(2), Some(2), Some(0)));
                cmd.flush_states();
                self.submit_stage(cmd, stream, view, RenderStage::ShadowGenerate);
            }
        }

        cmd.set_depth_mode(DepthMode::Reverse);
    }

    pub fn submit_stage(
        &self,
        cmd: &mut CommandList,
        stream: &FrameCommandStream,
        view: &ViewPacket,
        stage: RenderStage,
    ) {
        let block = stream.begin_parallel(cmd);
        let context = TempSubmitContext {
            renderer: self.parent.clone(),
            block: block.clone(),
            frame_packet: &self.frame_packet,
        };
        cmd.begin_event_str(stage.to_string());

        let range = 0..view.submit_node_blocks.block(stage).len();

        let mut job_handles = vec![];
        for chunk in RangeChunks::new(range, 64) {
            let ctx = context.clone();
            let view_id = view.id;
            let h = SCHEDULER
                .job_builder("scene_submit_parallel")
                .spawn(move || {
                    let ctx = ctx;
                    let mut cmd = ctx.block.cmd();
                    let render_objects = ctx.renderer.objects.read();

                    let frame_packet = unsafe { &*ctx.frame_packet };
                    let view = &frame_packet.views[view_id];
                    for submit_node in &view.submit_node_blocks.block(stage)[chunk] {
                        let view_node = &view.view_nodes[submit_node.view_node];
                        let frame_node = &frame_packet.per_frame_nodes[view_node.frame_node];
                        let Some(render_object) = render_objects.get(frame_node.object) else {
                            error!(
                                "Render object with handle {:?} not found",
                                frame_node.object
                            );
                            continue;
                        };
                        render_object.renderer.submit(
                            &mut cmd,
                            stage,
                            frame_node,
                            view_node,
                            submit_node.key,
                        );
                    }
                });
            job_handles.push(h);
        }

        let sync_job = SCHEDULER
            .job_builder("scene_submit_parallel_sync")
            .dependencies(job_handles)
            .spawn(|| {});

        sync_job.wait();

        stream.end_parallel(block);

        cmd.end_event();
    }

    pub fn submit_stage_serial(
        &self,
        cmd: &mut CommandList,
        stream: &FrameCommandStream,
        view: &ViewPacket,
        stage: RenderStage,
    ) {
        let block = stream.begin_parallel(cmd);
        let context = TempSubmitContext {
            renderer: self.parent.clone(),
            block: block.clone(),
            frame_packet: &self.frame_packet,
        };
        cmd.begin_event_str(stage.to_string());

        let range = 0..view.submit_node_blocks.block(stage).len();

        let mut job_handles = vec![];
        for (i, chunk) in
            RangeChunks::new(range.clone(), range.len().div_ceil(SCHEDULER.num_workers()))
                .enumerate()
        {
            let ctx = context.clone();
            let h = SCHEDULER.job_builder("scene_submit_serial").spawn(move || {
                let ctx = ctx;
                let mut cmd = ctx.block.cmd_manual(i);
                let render_objects = ctx.renderer.objects.read();

                let frame_packet = unsafe { &*ctx.frame_packet };
                let view = &frame_packet.views[0];
                for submit_node in &view.submit_node_blocks.block(stage)[chunk] {
                    let view_node = &view.view_nodes[submit_node.view_node];
                    let frame_node = &frame_packet.per_frame_nodes[view_node.frame_node];
                    let Some(render_object) = render_objects.get(frame_node.object) else {
                        error!(
                            "Render object with handle {:?} not found",
                            frame_node.object
                        );
                        continue;
                    };
                    render_object.renderer.submit(
                        &mut cmd,
                        stage,
                        frame_node,
                        view_node,
                        submit_node.key,
                    );
                }
            });
            job_handles.push(h);
        }

        let sync_job = SCHEDULER
            .job_builder("scene_submit_serial_sync")
            .dependencies(job_handles)
            .spawn(|| {});

        sync_job.wait();

        stream.end_parallel(block);

        cmd.end_event();
    }

    /// Sets the value of the given global channel by ID
    /// Returns `Some` with the previous value if the channel exists, `None` otherwise
    pub fn set_global_channel_by_id(&mut self, id: u32, v: Vec4) -> Option<Vec4> {
        if let Some(pos) = self.parent.externs.global_ids.iter().position(|i| *i == id) {
            Some(std::mem::replace(&mut self.global_channels[pos], v))
        } else {
            None
        }
    }

    /// Sets the value of the given global channel by name, hashing the name to get its ID
    /// Returns `Some` with the previous value if the channel exists, `None` otherwise
    pub fn set_global_channel_by_name(&mut self, name: &str, v: Vec4) -> Option<Vec4> {
        self.set_global_channel_by_id(fnv1(name), v)
    }

    pub fn get_global_channel_by_name(&self, name: &str) -> Option<Vec4> {
        self.get_global_channel_by_id(fnv1(name))
    }

    pub fn get_global_channel_by_id(&self, id: u32) -> Option<Vec4> {
        self.parent
            .externs
            .global_ids
            .iter()
            .position(|i| *i == id)
            .map(|pos| self.global_channels[pos])
    }
}

#[derive(Clone)]
struct TempSubmitContext {
    renderer: Arc<Renderer>,
    block: Arc<ParallelCommandBlock>,
    frame_packet: *const FramePacket,
}

unsafe impl Send for TempSubmitContext {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugPipeline {
    LookDev,
    GlobalLightingShading,
    DeferredShading,
    DeferredShadingNoAtm,

    Albedo,
    Smoothness,
    Metalness,
    AmbientOcclusion,
    Emission,
    EmissionIntensity,
    Transmission,
    Overcoat,

    DepthEdges,
    WorldNormal,
    Overdraw,

    LightDiffuse,
    LightSpecular,
}

impl DebugPipeline {
    pub const fn is_shaded(&self) -> bool {
        matches!(
            self,
            Self::GlobalLightingShading | Self::DeferredShading | Self::DeferredShadingNoAtm
        )
    }

    pub const fn aa_enabled(&self) -> bool {
        self.is_shaded() || matches!(self, Self::DepthEdges | Self::WorldNormal)
    }
}
