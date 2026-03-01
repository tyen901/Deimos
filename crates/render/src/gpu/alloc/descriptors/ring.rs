use std::sync::atomic::AtomicUsize;

use d3d12::DescriptorHeapType;

pub struct DescriptorRing {
    descriptor_heap: d3d12::DescriptorHeap,

    increment_size: u32,
    cpu_handle_base: d3d12::CpuDescriptorHandle,
    gpu_handle_base: d3d12::GpuDescriptorHandle,

    num_descriptors: usize,
    head: AtomicUsize,
}

impl DescriptorRing {
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

        Ok(DescriptorRing {
            cpu_handle_base: descriptor_heap.cpu_descriptor_handle_for_heap_start(),
            gpu_handle_base: descriptor_heap.gpu_descriptor_handle_for_heap_start(),
            increment_size: device.descriptor_handle_increment_size(heap_type),
            descriptor_heap,
            num_descriptors: size,
            head: AtomicUsize::new(0),
        })
    }

    pub fn reset(&self) {
        self.head.store(0, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn allocate(&self, num_descriptors: usize) -> DescriptorRange {
        let start = self
            .head
            .fetch_add(num_descriptors, std::sync::atomic::Ordering::Relaxed);
        if start + num_descriptors > self.num_descriptors {
            panic!(
                "DescriptorRing out of descriptors ({}/{} descriptors)",
                start + num_descriptors,
                self.num_descriptors
            );
        }

        DescriptorRange {
            cpu_start: self.cpu_handle(start),
            gpu_start: self.gpu_handle(start),
            num_descriptors,
            increment_size: self.increment_size,
        }
    }

    pub fn allocate_one(&self) -> (d3d12::CpuDescriptorHandle, d3d12::GpuDescriptorHandle) {
        let range = self.allocate(1);
        (range.cpu_start, range.gpu_start)
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

    pub fn used(&self) -> usize {
        self.head.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn capacity(&self) -> usize {
        self.num_descriptors
    }
}

pub struct DescriptorRange {
    cpu_start: d3d12::CpuDescriptorHandle,
    gpu_start: d3d12::GpuDescriptorHandle,
    num_descriptors: usize,
    increment_size: u32,
}

impl DescriptorRange {
    pub fn gpu_handle(&self, offset: usize) -> d3d12::GpuDescriptorHandle {
        self.gpu_start.offset(offset, self.increment_size)
    }

    pub fn cpu_handle(&self, offset: usize) -> d3d12::CpuDescriptorHandle {
        self.cpu_start.offset(offset, self.increment_size)
    }

    pub fn len(&self) -> usize {
        self.num_descriptors
    }

    pub fn is_empty(&self) -> bool {
        self.num_descriptors == 0
    }
}
