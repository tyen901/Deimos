pub mod object;

use std::sync::Arc;

use slotmap::SlotMap;

use crate::{
    asset::AssetManager,
    gpu::Gpu,
    renderer::object::{RenderObject, RenderObjectHandle},
};

pub struct Renderer {
    pub gpu: Arc<Gpu>,
    pub objects: SlotMap<RenderObjectHandle, RenderObject>,
    pub asset_manager: AssetManager,
}

impl Renderer {
    pub fn new(gpu: Arc<Gpu>) -> Self {
        Self {
            asset_manager: AssetManager::new(&gpu),
            gpu,
            objects: SlotMap::with_key(),
        }
    }
}
