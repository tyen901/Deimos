use std::{mem::ManuallyDrop, sync::Arc};

use crate::gpu::Gpu;

/// Represents an owned GPU resource that will be automatically freed when dropped.
pub struct OwnedResource {
    gpu: Arc<Gpu>,
    resource: ManuallyDrop<gpu_allocator::d3d12::Resource>,
    current_state: d3d12::ResourceStates,
}

impl OwnedResource {
    pub fn new(
        gpu: Arc<Gpu>,
        resource: gpu_allocator::d3d12::Resource,
        current_state: d3d12::ResourceStates,
    ) -> Self {
        Self {
            gpu,
            current_state,
            resource: ManuallyDrop::new(resource),
        }
    }

    pub fn transition(
        &mut self,
        cmd: &d3d12::GraphicsCommandList,
        new_states: d3d12::ResourceStates,
    ) {
        cmd.resource_barriers(&[d3d12::ResourceBarrier::transition(
            self.resource(),
            0,
            self.current_state,
            new_states,
        )]);
        self.current_state = new_states;
    }

    pub fn resource(&self) -> &d3d12::Resource {
        self.resource.resource().as_ref()
    }

    pub fn size(&self) -> u64 {
        self.resource.size
    }
}

impl Drop for OwnedResource {
    fn drop(&mut self) {
        let resource = unsafe { ManuallyDrop::take(&mut self.resource) };
        self.gpu
            .allocator
            .lock()
            .free_resource(resource)
            .expect("Failed to free resource");
    }
}
