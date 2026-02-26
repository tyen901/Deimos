use std::sync::Arc;

use anyhow::Context;
use d3d12::{
    D3D12_RESOURCE_DESC, D3D12_RESOURCE_DIMENSION_BUFFER, D3D12_RESOURCE_STATE_COMMON,
    D3D12_RESOURCE_STATE_GENERIC_READ, D3D12_TEXTURE_LAYOUT_ROW_MAJOR, GpuVirtualAddress,
    ID3D12Resource, Resource,
};
use gpu_allocator::{
    MemoryLocation,
    d3d12::{ResourceCategory, ResourceCreateDesc, ResourceStateOrBarrierLayout, ResourceType},
};

use crate::gpu::Gpu;

pub struct Buffer {
    gpu: Arc<Gpu>,
    // gpu_allocator's Resource owns both the ID3D12Resource and the Allocation
    resource: Option<gpu_allocator::d3d12::Resource>,
    mapped_ptr: Option<*mut u8>,
    size: u64,
}

// SAFETY: The mapped pointer is only written from one thread at a time (per-frame).
// The ID3D12Resource and Allocation are refcounted COM objects / heap-backed.
unsafe impl Send for Buffer {}
unsafe impl Sync for Buffer {}

impl Buffer {
    /// Create a GPU-only buffer (not CPU-accessible).
    /// Data must be uploaded via a copy command from an upload buffer.
    pub fn new_gpu(gpu: &Arc<Gpu>, size: u64, name: &str) -> anyhow::Result<Self> {
        Self::new_impl(gpu, size, MemoryLocation::GpuOnly, name)
    }

    /// Create a CPU-writable upload buffer.
    /// Stays mapped for its entire lifetime — write through `mapped_slice_mut()`.
    pub fn new_upload(gpu: &Arc<Gpu>, size: u64, name: &str) -> anyhow::Result<Self> {
        let mut buf = Self::new_impl(gpu, size, MemoryLocation::CpuToGpu, name)?;

        // Map the upload buffer once and keep it mapped (standard D3D12 pattern)
        let mut mapped_ptr = std::ptr::null_mut();
        unsafe {
            buf.d3d12_resource_win()
                .Map(0, None, Some(&mut mapped_ptr))
                .context("Failed to map upload buffer")?;
        }
        buf.mapped_ptr = Some(mapped_ptr as *mut u8);

        Ok(buf)
    }

    fn new_impl(
        gpu: &Arc<Gpu>,
        size: u64,
        location: MemoryLocation,
        name: &str,
    ) -> anyhow::Result<Self> {
        let resource_desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Width: size,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: d3d12::Format::Unknown.into(),
            SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            ..Default::default()
        };

        let initial_state = match location {
            MemoryLocation::CpuToGpu => D3D12_RESOURCE_STATE_GENERIC_READ,
            _ => D3D12_RESOURCE_STATE_COMMON,
        };

        let resource = gpu.allocator.lock().create_resource(&ResourceCreateDesc {
            name,
            memory_location: location,
            resource_category: ResourceCategory::Buffer,
            resource_desc: &resource_desc,
            castable_formats: &[],
            clear_value: None,
            initial_state_or_layout: ResourceStateOrBarrierLayout::ResourceState(initial_state),
            resource_type: &ResourceType::Placed,
        })?;

        Ok(Self {
            gpu: Arc::clone(gpu),
            resource: Some(resource),
            mapped_ptr: None,
            size,
        })
    }

    fn d3d12_resource_win(&self) -> &ID3D12Resource {
        self.resource.as_ref().unwrap().resource()
    }

    pub fn d3d12_resource(&self) -> &Resource {
        self.d3d12_resource_win().as_ref()
    }

    pub fn gpu_virtual_address(&self) -> GpuVirtualAddress {
        self.d3d12_resource().gpu_virtual_address()
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    /// Write data into an upload buffer. Panics if this is not an upload buffer.
    pub fn write(&self, offset: u64, data: &[u8]) {
        let ptr = self
            .mapped_ptr
            .expect("Buffer::write called on a non-upload buffer");
        assert!(
            offset + data.len() as u64 <= self.size,
            "Write out of bounds"
        );
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr.add(offset as usize), data.len());
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        // Unmap if mapped
        if self.mapped_ptr.is_some() {
            unsafe {
                self.d3d12_resource_win().Unmap(0, None);
            }
            self.mapped_ptr = None;
        }
        // Return the resource + allocation to the allocator
        if let Some(resource) = self.resource.take()
            && let Err(e) = self.gpu.allocator.lock().free_resource(resource)
        {
            error!("Failed to free buffer: {:?}", e);
        }
    }
}
