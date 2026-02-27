use std::sync::Arc;

use deimos_render::{
    asset::{Handle, vertex_buffer::VertexBuffer},
    renderer::{Renderer, object::RenderObjectHandle},
};

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

pub fn s_extract_render_objects(world: &hecs::World) {
    // for (_entity, static_render_object) in world.query::<&StaticRenderObject>().iter() {
    //     frame_packet.push_static_render_object(static_render_object.handle);
    // }

    // for (_entity, (transform, render_object, permutations)) in world
    //     .query::<(
    //         Option<&Transform>,
    //         &DynamicRenderObject,
    //         Option<&PermutationConfig>,
    //     )>()
    //     .iter()
    // {
    //     let transform = transform.copied().unwrap_or_default();
    //     let permutation = if let Some(permutation) = permutations {
    //         permutation
    //             .calculate_permutation_index()
    //             .unwrap_or(render_object.permutation)
    //     } else {
    //         render_object.permutation
    //     };

    //     frame_packet.push_dynamic_render_object(
    //         render_object.handle,
    //         transform.local_to_world().into(),
    //         permutation,
    //     );
    // }
}

pub fn s_are_all_objects_loaded(world: &hecs::World, renderer: &Renderer) -> bool {
    warn!("s_are_all_objects_loaded is not implemented yet");
    // for (_entity, static_render_object) in world.query::<&StaticRenderObject>().iter() {
    //     if !renderer.is_object_loaded(static_render_object.handle) {
    //         return false;
    //     }
    // }

    // for (_entity, render_object) in world.query::<&DynamicRenderObject>().iter() {
    //     if !renderer.is_object_loaded(render_object.handle) {
    //         return false;
    //     }
    // }

    true
}
