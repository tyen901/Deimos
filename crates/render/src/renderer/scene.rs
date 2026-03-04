//! This module provides view and frame packet management for the renderer.
//!
//! You can think of it as a "sub-renderer" that can exist alongside other sub-renderers and the main renderer without state mucking.

use std::sync::Arc;

use glam::Vec4;

use crate::{
    renderer::{Renderer, packet::FramePacket},
    tfx::view::ShadedView,
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
}
