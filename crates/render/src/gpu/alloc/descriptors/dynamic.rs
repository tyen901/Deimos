use d3d12::DescriptorHeapType;

pub struct DescriptorHeapAllocator {
    device: d3d12::Device,
    descriptor_heap: d3d12::DescriptorHeap,
    free_list: Vec<usize>,
    descriptor_labels: Vec<Option<String>>,

    increment_size: u32,
    cpu_handle_base: d3d12::CpuDescriptorHandle,

    capacity: usize,
    used: usize,
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
            device: device.clone(),
            cpu_handle_base: descriptor_heap.cpu_descriptor_handle_for_heap_start(),
            increment_size: device.descriptor_handle_increment_size(heap_type),
            null_texture2d,
            descriptor_heap,
            free_list: Vec::with_capacity(1024),
            descriptor_labels: Vec::with_capacity(1024),

            capacity: size,
            used: 1,
            head: 1,
        })
    }

    pub fn heap(&self) -> &d3d12::DescriptorHeap {
        &self.descriptor_heap
    }

    fn handle_for_index(&self, offset: usize) -> d3d12::CpuDescriptorHandle {
        self.cpu_handle_base.offset(offset, self.increment_size)
    }

    fn index_for_handle(&self, handle: d3d12::CpuDescriptorHandle) -> usize {
        handle.index(self.cpu_handle_base, self.increment_size)
    }

    fn allocate_handle(&mut self) -> d3d12::CpuDescriptorHandle {
        let handle = if let Some(index) = self.free_list.pop() {
            self.handle_for_index(index)
        } else {
            let index = self.head;
            self.head += 1;
            if index > self.capacity {
                panic!("Descriptor heap out of slots! ({} total)", self.capacity);
            }

            self.handle_for_index(index)
        };

        self.used += 1;

        handle
    }

    fn free_handle(&mut self, handle: d3d12::CpuDescriptorHandle) {
        let index = self.index_for_handle(handle);
        self.descriptor_labels[index] = None;
        self.free_list.push(index);

        self.used -= 1;
    }

    pub fn allocate_srv(
        &mut self,
        name: impl Into<String>,
        resource: &d3d12::Resource,
        srv_desc: &d3d12::ShaderResourceViewDesc,
    ) -> ResourceView {
        let name = name.into();
        let handle = self.allocate_handle();

        self.device
            .create_shader_resource_view(Some(resource), Some(srv_desc), handle);

        let index = self.index_for_handle(handle);
        if index >= self.descriptor_labels.len() {
            self.descriptor_labels.resize(index + 1, None);
        }
        self.descriptor_labels.insert(index, Some(name));

        ResourceView(handle)
    }

    pub fn free_srv(&mut self, handle: ResourceView) {
        self.free_handle(handle.0);
    }

    pub fn null(&self) -> ResourceView {
        ResourceView(self.null_texture2d)
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn used(&self) -> usize {
        self.used
    }
}

impl Drop for DescriptorHeapAllocator {
    fn drop(&mut self) {
        for (i, label) in self.descriptor_labels.iter().enumerate() {
            if let Some(label) = label {
                warn!("Descriptor 0x{i:X} ('{label}') was not freed");
            }
        }
    }
}

/// Texture view descriptor allocation.
///
/// This is internally represented by a 64-bit CPU descriptor handle, so copies are cheap.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct ResourceView(d3d12::CpuDescriptorHandle);

impl ResourceView {
    /// Returns the underlying CPU descriptor handle.
    pub fn handle(&self) -> d3d12::CpuDescriptorHandle {
        self.0
    }
}
