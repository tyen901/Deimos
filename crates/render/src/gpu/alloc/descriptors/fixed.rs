use d3d12::DescriptorHeapType;

pub struct FixedDescriptorHeap {
    descriptor_heap: d3d12::DescriptorHeap,

    increment_size: u32,
    cpu_handle_base: d3d12::CpuDescriptorHandle,
    gpu_handle_base: d3d12::GpuDescriptorHandle,
}

impl FixedDescriptorHeap {
    pub fn new(
        device: &d3d12::Device,
        heap_type: DescriptorHeapType,
        size: usize,
        shader_visible: bool,
    ) -> d3d12::Result<Self> {
        let descriptor_heap =
            device.create_descriptor_heap(heap_type, size as u32, shader_visible, 0)?;
        let null_texture2d = descriptor_heap.cpu_descriptor_handle_for_heap_start();
        device.create_shader_resource_view(
            None,
            Some(&d3d12::ShaderResourceViewDesc::texture_2d(
                d3d12::Format::R8g8b8a8Unorm,
                0,
                1,
                0.0,
                0,
            )),
            null_texture2d,
        );

        Ok(FixedDescriptorHeap {
            cpu_handle_base: descriptor_heap.cpu_descriptor_handle_for_heap_start(),
            gpu_handle_base: descriptor_heap.gpu_descriptor_handle_for_heap_start(),
            increment_size: device.descriptor_handle_increment_size(heap_type),
            descriptor_heap,
        })
    }

    pub fn heap(&self) -> &d3d12::DescriptorHeap {
        &self.descriptor_heap
    }

    pub fn gpu_handle(&self, offset: usize) -> d3d12::GpuDescriptorHandle {
        self.gpu_handle_base.offset(offset, self.increment_size)
    }

    pub fn cpu_handle(&self, offset: usize) -> d3d12::CpuDescriptorHandle {
        self.cpu_handle_base.offset(offset, self.increment_size)
    }

    pub fn cpu_index(&self, handle: d3d12::CpuDescriptorHandle) -> usize {
        handle.index(self.cpu_handle_base, self.increment_size)
    }
}
