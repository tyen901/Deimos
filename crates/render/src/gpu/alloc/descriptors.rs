use d3d12::DescriptorHeapType;

pub struct DescriptorHeapAllocator {
    pub descriptor_heap: d3d12::DescriptorHeap,
    free_list: Vec<usize>,

    increment_size: u32,
    cpu_handle_base: d3d12::CpuDescriptorHandle,

    size: usize,
    head: usize,

    pub null_texture2d: d3d12::CpuDescriptorHandle,
}

impl DescriptorHeapAllocator {
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

        Ok(DescriptorHeapAllocator {
            cpu_handle_base: descriptor_heap.cpu_descriptor_handle_for_heap_start(),
            increment_size: device.descriptor_handle_increment_size(heap_type),
            null_texture2d,
            descriptor_heap,
            free_list: Vec::with_capacity(1024),

            size,
            head: 1,
        })
    }

    fn handle_for_index(&self, offset: usize) -> d3d12::CpuDescriptorHandle {
        self.cpu_handle_base.offset(offset, self.increment_size)
    }

    fn index_for_handle(&self, handle: d3d12::CpuDescriptorHandle) -> usize {
        handle.index(self.cpu_handle_base, self.increment_size)
    }

    pub fn allocate(&mut self) -> d3d12::CpuDescriptorHandle {
        if let Some(index) = self.free_list.pop() {
            self.handle_for_index(index)
        } else {
            let index = self.head;
            self.head += 1;
            if index > self.size {
                panic!("Descriptor heap out of slots! ({} total)", self.size);
            }

            self.handle_for_index(index)
        }
    }

    pub fn free(&mut self, handle: d3d12::CpuDescriptorHandle) {
        let index = self.index_for_handle(handle);
        self.free_list.push(index);
    }
}
