//! This module provides view and frame packet management for the renderer.
//!
//! You can think of it as a "sub-renderer" that can exist alongside other sub-renderers and the main renderer without state mucking.

use std::sync::Arc;

use deimos_core::job::SCHEDULER;
use deimos_data::tfx::RenderStage;
use glam::Vec4;

use crate::{
    gpu::{
        command_list::CommandList,
        stream::{FrameCommandStream, ParallelCommandBlock},
    },
    renderer::{
        Renderer,
        packet::{FramePacket, ViewPacket},
    },
    tfx::view::ShadedView,
    util::range::RangeChunks,
};

pub struct SceneRenderer {
    pub parent: Arc<Renderer>,

    pub frame_packet: FramePacket,

    pub main_view: ShadedView,

    pub global_channels: [Vec4; 256],
}

impl SceneRenderer {
    pub fn new(parent: Arc<Renderer>) -> anyhow::Result<Self> {
        Ok(Self {
            frame_packet: FramePacket::default(),
            main_view: ShadedView::new(&parent.gpu, (1920, 1080))?,
            global_channels: parent.globals.channels.default_values(),
            parent,
        })
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

        let range = 0..view.submit_node_blocks.block(stage).len();

        let mut job_handles = vec![];
        for chunk in RangeChunks::new(range, 256) {
            let ctx = context.clone();
            let h = SCHEDULER
                .job_builder("scene_submit_parallel")
                .spawn(move || {
                    let ctx = ctx;
                    let mut cmd = ctx.block.cmd();
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
            .job_builder("scene_submit_parallel_sync")
            .dependencies(job_handles)
            .spawn(|| {});

        sync_job.wait();

        stream.end_parallel(block);
    }
}

#[derive(Clone)]
struct TempSubmitContext {
    renderer: Arc<Renderer>,
    block: Arc<ParallelCommandBlock>,
    frame_packet: *const FramePacket,
}

unsafe impl Send for TempSubmitContext {}
