use std::{ops::Range, sync::Arc};

use anyhow::Context;
use d3d12::{
    BufferSrvFlags, D3D12_DEFAULT_RESOURCE_PLACEMENT_ALIGNMENT, D3D12_RESOURCE_DESC,
    D3D12_RESOURCE_DIMENSION_BUFFER, D3D12_RESOURCE_STATE_COMMON,
    D3D12_RESOURCE_STATE_GENERIC_READ, D3D12_TEXTURE_DATA_PLACEMENT_ALIGNMENT,
    D3D12_TEXTURE_LAYOUT_ROW_MAJOR, GpuVirtualAddress, ID3D12Resource, Resource, ResourceBarrier,
    ResourceStates, error::D3DResultExt,
};
use deimos_data::tfx::ShaderStage;
use gpu_allocator::{
    MemoryLocation,
    d3d12::{ResourceCategory, ResourceCreateDesc, ResourceStateOrBarrierLayout, ResourceType},
};

use crate::gpu::{
    Gpu,
    alloc::{descriptors::ResourceView, resource::OwnedResource},
    command_list::CommandList,
};

pub struct DynamicBuffer {
    gpu: Arc<Gpu>,
    // gpu_allocator's Resource owns both the ID3D12Resource and the Allocation
    resource: Option<gpu_allocator::d3d12::Resource>,
    mapped_ptr: Option<*mut u8>,
    size: u64,
}

// SAFETY: The mapped pointer is only written from one thread at a time (per-frame).
// The ID3D12Resource and Allocation are refcounted COM objects / heap-backed.
unsafe impl Send for DynamicBuffer {}
unsafe impl Sync for DynamicBuffer {}

impl DynamicBuffer {
    /// Create a GPU-only buffer (not CPU-accessible).
    /// Data must be uploaded via a copy command from an upload buffer.
    pub fn new_gpu(gpu: &Arc<Gpu>, size: u64, name: &str) -> anyhow::Result<Self> {
        Self::new_impl(gpu, size, MemoryLocation::GpuOnly, name)
    }

    /// Create a CPU-writable upload buffer.
    /// Stays mapped for the duration of it's lifetime
    pub fn new_upload(gpu: &Arc<Gpu>, size: u64, name: &str) -> anyhow::Result<Self> {
        let mut buf = Self::new_impl(gpu, size, MemoryLocation::CpuToGpu, name)?;

        // Map the upload buffer once and keep it mapped (standard D3D12 pattern)
        let mut mapped_ptr = std::ptr::null_mut();
        unsafe {
            buf.d3d12_resource_win()
                .Map(0, None, Some(&mut mapped_ptr))
                .context("Failed to map upload buffer")?;
        }
        buf.mapped_ptr = Some(mapped_ptr.cast::<u8>());

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

        let resource = gpu
            .allocator
            .lock()
            .create_resource(&ResourceCreateDesc {
                name,
                memory_location: location,
                resource_category: ResourceCategory::Buffer,
                resource_desc: &resource_desc,
                castable_formats: &[],
                clear_value: None,
                initial_state_or_layout: ResourceStateOrBarrierLayout::ResourceState(initial_state),
                resource_type: &ResourceType::Placed,
            })
            .catch_device_removal(gpu)?;

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

    pub const fn size(&self) -> u64 {
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

    pub fn vb_view(&self, stride: u32) -> d3d12::VertexBufferView {
        d3d12::VertexBufferView::new(self.gpu_virtual_address(), self.size as u32, stride)
    }
}

impl Drop for DynamicBuffer {
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

/// Immutable GPU-only buffer
pub struct ImmutableBuffer {
    gpu: Arc<Gpu>,
    resource: OwnedResource,
    srv: ResourceView,
    size: u64,
    view_format: d3d12::Format,
}

impl ImmutableBuffer {
    pub fn new(
        gpu: &Arc<Gpu>,
        name: &str,
        view_format: d3d12::Format,
        data: &[u8],
    ) -> anyhow::Result<Self> {
        if data.len() >= 24899584 {
            println!("Large buffer found, dumping");
            std::fs::write("large_buffer.bin", data)?;
        }

        let resource_desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Width: data.len() as u64,
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

        let resource = gpu.allocate_resource(&ResourceCreateDesc {
            name,
            memory_location: MemoryLocation::GpuOnly,
            resource_category: ResourceCategory::Buffer,
            resource_desc: &resource_desc,
            castable_formats: &[],
            clear_value: None,
            initial_state_or_layout: ResourceStateOrBarrierLayout::ResourceState(
                d3d12::D3D12_RESOURCE_STATE_COMMON,
            ),
            resource_type: &ResourceType::Placed,
        })?;

        let upload_buffer = gpu.allocate_upload_buffer(data.len() as u64)?;
        unsafe {
            let dst_base = upload_buffer.resource().map(0)?;

            dst_base.copy_from(data.as_ptr(), data.len());

            upload_buffer.resource().unmap(0);
        }

        gpu.cmd_scope(|cmd| {
            cmd.resource_barriers(&[ResourceBarrier::transition(
                resource.resource(),
                0,
                ResourceStates::COMMON,
                ResourceStates::COPY_DEST,
            )]);

            cmd.copy_resource(upload_buffer.resource(), resource.resource());

            cmd.resource_barriers(&[ResourceBarrier::transition(
                resource.resource(),
                0,
                ResourceStates::COPY_DEST,
                ResourceStates::ALL_SHADER_RESOURCE,
            )]);

            Ok(())
        })
        .context("buffer copy")?;

        let srv = gpu.resource_heap.lock().allocate_srv(
            name,
            resource.resource(),
            &d3d12::ShaderResourceViewDesc::buffer(
                view_format,
                0..view_format.buffer_element_count(data.len() as u64),
                0,
                BufferSrvFlags::empty(),
            ),
        );

        Ok(Self {
            gpu: Arc::clone(gpu),
            resource,
            srv,
            view_format,
            size: data.len() as u64,
        })
    }

    pub fn create_srv(&self, gpu: &Gpu, bytes_range: Range<u64>) -> ResourceView {
        gpu.resource_heap.lock().allocate_srv(
            "immutable_buffer_slice",
            self.resource.resource(),
            &d3d12::ShaderResourceViewDesc::buffer(
                self.view_format,
                self.view_format.buffer_element_count(bytes_range.start)
                    ..self.view_format.buffer_element_count(bytes_range.end),
                0,
                BufferSrvFlags::empty(),
            ),
        )
    }

    pub const fn size(&self) -> u64 {
        self.size
    }

    pub const fn resource_view(&self) -> ResourceView {
        self.srv
    }

    pub fn constant_buffer_view(&self) -> d3d12::GpuVirtualAddress {
        self.resource.resource().gpu_virtual_address()
    }

    pub fn bind_srv(&self, cmd: &mut CommandList, stage: ShaderStage, slot: u32) {
        cmd.set_shader_resource_view(stage, slot, Some(self.resource_view()));
    }

    pub fn bind_cbv(&self, cmd: &mut CommandList, stage: ShaderStage, slot: u32) {
        cmd.set_shader_constant_buffer_view(stage, slot, Some(self.constant_buffer_view()));
    }
}

impl Drop for ImmutableBuffer {
    fn drop(&mut self) {
        self.gpu.resource_heap.lock().free_srv(self.srv);
    }
}
