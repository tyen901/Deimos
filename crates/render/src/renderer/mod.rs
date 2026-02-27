pub mod globals;
pub mod object;

use std::sync::Arc;

use parking_lot::RwLock;
use slotmap::SlotMap;

use crate::{
    asset::AssetManager,
    gpu::Gpu,
    renderer::{
        globals::RenderGlobals,
        object::{RenderObject, RenderObjectHandle},
    },
    tfx::externs::ExternContainer,
    util::thread_cell::ThreadMutCell,
};

pub struct Renderer {
    pub gpu: Arc<Gpu>,
    pub objects: RwLock<SlotMap<RenderObjectHandle, RenderObject>>,
    pub asset_manager: AssetManager,
    pub globals: RenderGlobals,
    pub externs: ThreadMutCell<ExternContainer>,
}

impl Renderer {
    pub fn new(gpu: Arc<Gpu>) -> Self {
        let asset_manager = AssetManager::new(&gpu);
        Self {
            globals: RenderGlobals::load(&asset_manager, &gpu)
                .expect("Failed to load render globals"),
            asset_manager,
            gpu,
            objects: RwLock::new(SlotMap::with_key()),
            externs: ThreadMutCell::new(ExternContainer::default()),
        }
    }

    pub fn shutdown(&self) {
        self.objects.write().clear();
        self.asset_manager.shutdown();
        self.gpu.shutdown();
    }
}

impl Renderer {
    pub fn add_object(&self, object: RenderObject) -> RenderObjectHandle {
        self.objects.write().insert(object)
    }

    pub fn remove_object(&self, handle: RenderObjectHandle) {
        self.objects.write().remove(handle);
    }
}
