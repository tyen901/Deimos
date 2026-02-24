use std::sync::atomic::AtomicUsize;

use d3d12::{HeapFlags, HeapProperties};

/// Lock-free thread-safe ring buffer for uploading data to the GPU.
pub struct UploadRing {
    heap_resource: d3d12::Resource,
    mapped_ptr: *mut u8,

    gpu_base: d3d12::GpuVirtualAddress,
    head: AtomicUsize,
    capacity: usize,
}

impl UploadRing {
    pub fn new(device: &d3d12::Device, capacity: u64) -> anyhow::Result<Self> {
        let heap_resource = device.create_committed_resource(
            &HeapProperties::new(d3d12::HeapType::Upload),
            HeapFlags::empty(),
            &d3d12::ResourceDesc::buffer(capacity),
            d3d12::ResourceStates::GENERIC_READ,
        )?;

        let gpu_base = heap_resource.gpu_virtual_address();
        let mapped_ptr = heap_resource.map(0)?;

        Ok(Self {
            heap_resource,
            mapped_ptr,
            gpu_base,
            head: AtomicUsize::new(0),
            capacity: capacity as usize,
        })
    }

    pub fn reset(&self) {
        self.head.store(0, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn alloc(&self, size: usize) -> anyhow::Result<RingSlice> {
        let size_aligned = size.next_multiple_of(256);
        let start = self
            .head
            .fetch_add(size_aligned, std::sync::atomic::Ordering::Relaxed);

        debug_assert!(start.is_multiple_of(256), "Misaligned ring buffer");
        if start + size_aligned > self.capacity {
            Err(anyhow::anyhow!("Out of memory"))
        } else {
            Ok(RingSlice {
                ptr: unsafe { self.mapped_ptr.add(start) },
                len: size,
                capacity: size_aligned,
            })
        }
    }
}

pub struct RingSlice {
    ptr: *const u8,
    len: usize,
    capacity: usize,
}

impl RingSlice {
    pub fn ptr(&self) -> *const u8 {
        self.ptr
    }

    /// Returns the total capacity of the ring slice.
    ///
    /// This is the allocated length aligned up to 256 bytes
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the length of the ring slice.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}
