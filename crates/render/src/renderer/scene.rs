//! This module provides view and frame packet management for the renderer.
//!
//! You can think of it as a "sub-renderer" that can exist alongside other sub-renderers and the main renderer without state mucking.

use std::sync::Arc;

use crate::{
    renderer::{Renderer, packet::FramePacket},
    tfx::view::ShadedView,
};

pub struct SceneRenderer {
    pub parent: Arc<Renderer>,

    pub frame_packet: FramePacket,

    pub main_view: ShadedView,
}
