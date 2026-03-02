use std::sync::atomic::{AtomicUsize, Ordering};

use d3d12::{D3D12_CONSTANT_BUFFER_DATA_PLACEMENT_ALIGNMENT, HeapFlags, HeapProperties};

/// Lock-free thread-safe ring buffer for uploading data to the GPU.
pub struct UploadRing {
    _heap_resource: d3d12::Resource,
    mapped_ptr: *mut u8,

    gpu_base: d3d12::GpuVirtualAddress,
    head: AtomicUsize,
    capacity: usize,
    num_allocations: AtomicUsize,
}

impl UploadRing {
    const BLOCK_ALIGNMENT: usize = D3D12_CONSTANT_BUFFER_DATA_PLACEMENT_ALIGNMENT as usize;

    pub fn new(device: &d3d12::Device, capacity: u64) -> anyhow::Result<Self> {
        let heap_resource = device.create_committed_resource(
            &HeapProperties::new(d3d12::HeapType::Upload),
            HeapFlags::empty(),
            &d3d12::ResourceDesc::buffer(capacity),
            d3d12::ResourceStates::GENERIC_READ,
        )?;

        let gpu_base = heap_resource.gpu_virtual_address();
        let mapped_ptr = heap_resource.map(0)?;
        unsafe {
            mapped_ptr.write_bytes(0xCC, Self::BLOCK_ALIGNMENT);
        }

        Ok(Self {
            _heap_resource: heap_resource,
            mapped_ptr,
            gpu_base,
            // First block is reserved for null pointer
            head: AtomicUsize::new(Self::BLOCK_ALIGNMENT),
            capacity: capacity as usize,
            num_allocations: AtomicUsize::new(0),
        })
    }

    pub fn reset(&self) {
        self.head.store(Self::BLOCK_ALIGNMENT, Ordering::Relaxed);
        self.num_allocations.store(0, Ordering::Relaxed);
    }

    pub fn alloc_slice(&self, size: usize) -> anyhow::Result<RingSlice> {
        let size_aligned = size.next_multiple_of(Self::BLOCK_ALIGNMENT);
        let start = self.head.fetch_add(size_aligned, Ordering::Relaxed);
        self.num_allocations.fetch_add(1, Ordering::Relaxed);

        debug_assert!(
            start.is_multiple_of(Self::BLOCK_ALIGNMENT),
            "Misaligned ring buffer"
        );
        if start + size_aligned > self.capacity {
            Err(anyhow::anyhow!("Out of memory"))
        } else {
            Ok(RingSlice {
                ptr: unsafe { self.mapped_ptr.add(start) },
                len: size,
                capacity: size_aligned,
                gpu_va: self.gpu_base.offset(start),
            })
        }
    }

    pub fn alloc<T: Sized>(&self) -> anyhow::Result<TypedRingAllocation<T>> {
        let slice = self.alloc_slice(std::mem::size_of::<T>())?;
        Ok(TypedRingAllocation {
            ptr: slice.ptr as *mut T,
            gpu_va: slice.gpu_va,
        })
    }

    pub fn null(&self) -> d3d12::GpuVirtualAddress {
        self.gpu_base
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn used(&self) -> usize {
        self.head.load(Ordering::Relaxed)
    }

    pub fn num_allocations(&self) -> usize {
        self.num_allocations.load(Ordering::Relaxed)
    }
}

pub struct RingSlice {
    ptr: *mut u8,
    len: usize,
    capacity: usize,

    gpu_va: d3d12::GpuVirtualAddress,
}

impl RingSlice {
    pub fn ptr(&self) -> *mut u8 {
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

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    pub fn virtual_address(&self) -> d3d12::GpuVirtualAddress {
        self.gpu_va
    }
}

pub struct TypedRingAllocation<T: Sized> {
    ptr: *mut T,
    gpu_va: d3d12::GpuVirtualAddress,
}

impl<T: Sized> TypedRingAllocation<T> {
    pub fn ptr(&self) -> *mut T {
        self.ptr
    }

    pub fn write(&self, data: &T) {
        unsafe {
            self.ptr().copy_from(data as *const T, 1);
        }
    }

    pub fn virtual_address(&self) -> d3d12::GpuVirtualAddress {
        self.gpu_va
    }
}
