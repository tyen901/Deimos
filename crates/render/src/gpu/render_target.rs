use std::sync::Arc;

use d3d12::ResourceFlags;

use crate::gpu::{Gpu, alloc::resource::OwnedResource};

pub struct DepthBuffer {
    pub resource: OwnedResource,
    size: (u32, u32),
    dsv_heap: d3d12::DescriptorHeap,
    gpu: Arc<Gpu>,
}

impl DepthBuffer {
    pub fn new(gpu: &Arc<Gpu>, (width, height): (u32, u32)) -> anyhow::Result<Self> {
        let resource = gpu.allocate_resource(&gpu_allocator::d3d12::ResourceCreateDesc {
            name: "depth_buffer",
            memory_location: gpu_allocator::MemoryLocation::GpuOnly,
            resource_category: gpu_allocator::d3d12::ResourceCategory::RtvDsvTexture,
            resource_desc: d3d12::ResourceDesc::new(d3d12::ResourceDimension::Texture2D)
                .width(width as u64)
                .height(height)
                .format(d3d12::Format::R32FloatX8x24Typeless)
                .flags(ResourceFlags::ALLOW_DEPTH_STENCIL)
                .as_ref(),
            castable_formats: &[],
            clear_value: None,
            initial_state_or_layout:
                gpu_allocator::d3d12::ResourceStateOrBarrierLayout::ResourceState(
                    d3d12::D3D12_RESOURCE_STATE_DEPTH_WRITE,
                ),
            resource_type: &gpu_allocator::d3d12::ResourceType::Placed,
        })?;

        let rtv_heap = gpu.create_descriptor_heap(d3d12::DescriptorHeapType::Dsv, 1, false, 0)?;
        gpu.create_depth_stencil_view(
            Some(resource.resource()),
            Some(&d3d12::DepthStencilViewDesc::texture_2d(
                d3d12::Format::D32FloatS8x24Uint,
            )),
            rtv_heap.cpu_descriptor_handle_for_heap_start(),
        );

        Ok(Self {
            gpu: gpu.clone(),
            resource,
            dsv_heap: rtv_heap,
            size: (width, height),
        })
    }

    pub fn cpu_handle(&self) -> d3d12::CpuDescriptorHandle {
        self.dsv_heap.cpu_descriptor_handle_for_heap_start()
    }

    pub fn gpu_handle(&self) -> d3d12::GpuDescriptorHandle {
        self.dsv_heap.gpu_descriptor_handle_for_heap_start()
    }

    /// Resize the depth buffer.
    ///
    /// # Remarks
    /// - Won't resize if the new size is the same as the current size.
    /// - The old depth buffer is put into the GPU resource bin, meaning it won't actually be destroyed until it's no longer used
    pub fn resize(&mut self, (width, height): (u32, u32)) -> anyhow::Result<()> {
        if width == self.size.0 && height == self.size.1 {
            return Ok(());
        }

        let mut new_depth_buffer = Self::new(&self.gpu, (width, height))?;
        std::mem::swap(self, &mut new_depth_buffer);

        Ok(())
    }
}
