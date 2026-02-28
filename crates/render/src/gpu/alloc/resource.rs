use std::{mem::ManuallyDrop, sync::Arc};

use crate::gpu::Gpu;

/// Represents an owned GPU resource that will be automatically freed when dropped.
pub struct OwnedResource {
    guard: ManuallyDrop<ResourceGuard>,
}

impl OwnedResource {
    pub fn new(
        gpu: Arc<Gpu>,
        resource: gpu_allocator::d3d12::Resource,
        current_state: d3d12::ResourceStates,
    ) -> Self {
        Self {
            guard: ManuallyDrop::new(ResourceGuard {
                gpu,
                resource: ManuallyDrop::new(resource),
                current_state,
            }),
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
            self.guard.current_state,
            new_states,
        )]);
        self.guard.current_state = new_states;
    }

    pub fn resource(&self) -> &d3d12::Resource {
        self.guard.resource.resource().as_ref()
    }

    pub fn size(&self) -> u64 {
        self.guard.resource.size
    }
}

impl Drop for OwnedResource {
    fn drop(&mut self) {
        let gpu = self.guard.gpu.clone();
        unsafe {
            gpu.bin_resource(ManuallyDrop::take(&mut self.guard));
        }
    }
}

struct ResourceGuard {
    gpu: Arc<Gpu>,
    resource: ManuallyDrop<gpu_allocator::d3d12::Resource>,
    current_state: d3d12::ResourceStates,
}

impl Drop for ResourceGuard {
    fn drop(&mut self) {
        let resource = unsafe { ManuallyDrop::take(&mut self.resource) };
        self.gpu
            .allocator
            .lock()
            .free_resource(resource)
            .expect("Failed to free resource");
    }
}
