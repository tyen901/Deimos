use deimos_render::{Renderer, object::RenderObjectHandle, tfx::packet::FramePacket};

use crate::world::transform::Transform;

pub struct StaticRenderObject {
    handle: RenderObjectHandle,
}

impl StaticRenderObject {
    pub fn new(render_object: RenderObjectHandle) -> Self {
        Self {
            handle: render_object,
        }
    }
}

impl Drop for StaticRenderObject {
    fn drop(&mut self) {
        Renderer::instance().remove_object(self.handle);
    }
}

pub struct DynamicRenderObject {
    handle: RenderObjectHandle,
}

impl DynamicRenderObject {
    pub fn new(render_object: RenderObjectHandle) -> Self {
        Self {
            handle: render_object,
        }
    }
}

impl Drop for DynamicRenderObject {
    fn drop(&mut self) {
        Renderer::instance().remove_object(self.handle);
    }
}

pub fn s_extract_render_objects(world: &hecs::World, frame_packet: &mut FramePacket) {
    for (_entity, static_render_object) in world.query::<&StaticRenderObject>().iter() {
        frame_packet.push_static_render_object(static_render_object.handle);
    }

    for (_entity, (transform, render_object)) in world
        .query::<(Option<&Transform>, &DynamicRenderObject)>()
        .iter()
    {
        let transform = transform.copied().unwrap_or_default();
        frame_packet
            .push_dynamic_render_object(render_object.handle, transform.local_to_world().into());
    }
}
