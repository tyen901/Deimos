use std::sync::Arc;

use crate::renderer::{Renderer, object::RenderObjectHandle};

pub struct StaticRenderObject {
    renderer: Arc<Renderer>,
    handle: RenderObjectHandle,
}

impl StaticRenderObject {
    pub fn new(renderer: &Arc<Renderer>, render_object: RenderObjectHandle) -> Self {
        Self {
            renderer: renderer.clone(),
            handle: render_object,
        }
    }

    pub const fn handle(&self) -> RenderObjectHandle {
        self.handle
    }
}

impl Drop for StaticRenderObject {
    fn drop(&mut self) {
        self.renderer.remove_object(self.handle);
    }
}

pub struct DynamicRenderObject {
    renderer: Arc<Renderer>,
    handle: RenderObjectHandle,
    pub permutation: usize,
}

impl DynamicRenderObject {
    pub fn new(renderer: &Arc<Renderer>, render_object: RenderObjectHandle) -> Self {
        Self {
            renderer: renderer.clone(),
            handle: render_object,
            permutation: 0,
        }
    }

    pub const fn handle(&self) -> RenderObjectHandle {
        self.handle
    }
}

impl Drop for DynamicRenderObject {
    fn drop(&mut self) {
        self.renderer.remove_object(self.handle);
    }
}

// pub struct StaticAmbientOcclusion {
//     pub buffer: Handle<VertexBuffer>,
//     pub ao: SStaticAmbientOcclusion,
// }

// impl StaticAmbientOcclusion {
//     pub fn new(ao: SStaticAmbientOcclusion) -> Self {
//         let buffer = Renderer::instance().asset_manager.load(ao.ao0.buffer);
//         Self { buffer, ao }
//     }
// }

// pub fn s_extract_ambient_occlusion(world: &hecs::World) {
//     let renderer = Renderer::instance();
//     if let Some((_entity, ao)) = world.query::<&StaticAmbientOcclusion>().iter().next() {
//         *renderer.ao.write() = Some(ao.ao.clone());
//         *renderer.ao_buffer.write() = Some(ao.buffer.clone());
//     }
// }

// pub fn s_are_all_objects_loaded(_world: &hecs::World, _renderer: &Renderer) -> bool {
//     warn!("s_are_all_objects_loaded is not implemented yet");
//     // for (_entity, static_render_object) in world.query::<&StaticRenderObject>().iter() {
//     //     if !renderer.is_object_loaded(static_render_object.handle) {
//     //         return false;
//     //     }
//     // }

//     // for (_entity, render_object) in world.query::<&DynamicRenderObject>().iter() {
//     //     if !renderer.is_object_loaded(render_object.handle) {
//     //         return false;
//     //     }
//     // }

//     true
// }
