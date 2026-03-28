use std::{mem::ManuallyDrop, sync::Arc};

use crate::gpu::Gpu;

/// Represents an owned GPU resource that will be automatically freed when dropped.
pub struct OwnedResource {
    guard: ManuallyDrop<ResourceGuard>,
}

impl OwnedResource {
    pub fn new(
        gpu: Arc<Gpu>,
        category: gpu_allocator::d3d12::ResourceCategory,
        location: gpu_allocator::MemoryLocation,
        resource: gpu_allocator::d3d12::Resource,
        current_state: d3d12::ResourceStates,
    ) -> Self {
        let bytes = match category {
            gpu_allocator::d3d12::ResourceCategory::OtherTexture => {
                &gpu.num_bytes_allocated_for_textures
            }
            gpu_allocator::d3d12::ResourceCategory::RtvDsvTexture => {
                &gpu.num_bytes_allocated_for_render_targets
            }
            gpu_allocator::d3d12::ResourceCategory::Buffer => {
                if location == gpu_allocator::MemoryLocation::GpuOnly {
                    &gpu.num_bytes_allocated_for_buffers
                } else {
                    &gpu.num_bytes_allocated_for_transfer_buffers
                }
            }
        };

        bytes.fetch_add(resource.size as usize, std::sync::atomic::Ordering::Relaxed);

        Self {
            guard: ManuallyDrop::new(ResourceGuard {
                gpu,
                resource: ManuallyDrop::new(resource),
                current_state,
                category,
                location,
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
    category: gpu_allocator::d3d12::ResourceCategory,
    location: gpu_allocator::MemoryLocation,
}

impl Drop for ResourceGuard {
    fn drop(&mut self) {
        let resource = unsafe { ManuallyDrop::take(&mut self.resource) };

        let bytes = match self.category {
            gpu_allocator::d3d12::ResourceCategory::OtherTexture => {
                &self.gpu.num_bytes_allocated_for_textures
            }
            gpu_allocator::d3d12::ResourceCategory::RtvDsvTexture => {
                &self.gpu.num_bytes_allocated_for_render_targets
            }
            gpu_allocator::d3d12::ResourceCategory::Buffer => {
                if self.location != gpu_allocator::MemoryLocation::GpuOnly {
                    &self.gpu.num_bytes_allocated_for_transfer_buffers
                } else {
                    &self.gpu.num_bytes_allocated_for_buffers
                }
            }
        };

        bytes.fetch_sub(resource.size as usize, std::sync::atomic::Ordering::Relaxed);

        self.gpu
            .allocator
            .lock()
            .free_resource(resource)
            .expect("Failed to free resource");
    }
}
