pub mod globals;
pub mod object;

use std::sync::Arc;

use slotmap::SlotMap;

use crate::{
    asset::AssetManager,
    gpu::Gpu,
    renderer::{
        globals::RenderGlobals,
        object::{RenderObject, RenderObjectHandle},
    },
};

pub struct Renderer {
    pub gpu: Arc<Gpu>,
    pub objects: SlotMap<RenderObjectHandle, RenderObject>,
    pub asset_manager: AssetManager,
    pub globals: RenderGlobals,
}

impl Renderer {
    pub fn new(gpu: Arc<Gpu>) -> Self {
        Self {
            globals: RenderGlobals::load(&gpu).expect("Failed to load render globals"),
            asset_manager: AssetManager::new(&gpu),
            gpu,
            objects: SlotMap::with_key(),
        }
    }
}
