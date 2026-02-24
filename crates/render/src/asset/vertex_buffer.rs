use std::{sync::Arc, time::Instant};

use anyhow::Context;
use d3d12::DeviceChild;
use deimos_data::tfx::buffers::VertexBufferHeader;
use gpu_allocator::d3d12::{ResourceCreateDesc, ResourceType};
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use crate::gpu::{Gpu, alloc::resource::OwnedResource, command_list::CommandList};

pub struct VertexBuffer {
    resource: OwnedResource,

    pub size: u32,
    pub length: u32,
    pub stride: u32,
}

impl VertexBuffer {
    pub fn load(gpu: &Arc<Gpu>, hash: TagHash) -> anyhow::Result<Self> {
        let entry = package_manager()
            .get_entry(hash)
            .context("Entry not found")?;

        let header: VertexBufferHeader = package_manager()
            .read_tag_struct(hash)
            .context("Failed to read header data")?;
        let data = package_manager()
            .read_tag(entry.reference)
            .context("Failed to read buffer data")?;

        let vb = VertexBuffer::load_data(gpu, &data, header.stride as _)?;
        vb.resource
            .resource()
            .set_debug_name(format!("vertex_buffer {hash}"));
        Ok(vb)
    }

    pub fn load_data(gpu: &Arc<Gpu>, data: &[u8], stride: u32) -> anyhow::Result<Self> {
        Self::load_data_ex(gpu, data, stride)
    }

    #[profiling::function]
    pub fn load_data_ex(gpu: &Arc<Gpu>, data: &[u8], stride: u32) -> anyhow::Result<Self> {
        let desc = d3d12::ResourceDesc::buffer(data.len() as u64);

        let mut resource = gpu.allocate_resource(&ResourceCreateDesc {
            name: "vertex_buffer",
            memory_location: gpu_allocator::MemoryLocation::GpuOnly,
            resource_category: gpu_allocator::d3d12::ResourceCategory::Buffer,
            resource_desc: desc.as_ref(),
            castable_formats: &[],
            clear_value: None,
            initial_state_or_layout:
                gpu_allocator::d3d12::ResourceStateOrBarrierLayout::ResourceState(
                    d3d12::D3D12_RESOURCE_STATE_COMMON,
                ),
            resource_type: &ResourceType::Placed,
        })?;

        let mut upload_buffer = gpu.allocate_upload_buffer(data.len() as u64)?;
        let upload_buffer_data = upload_buffer
            .resource()
            .map(0)
            .context("Failed to map upload buffer")?;
        unsafe {
            upload_buffer_data.copy_from_nonoverlapping(data.as_ptr(), data.len());
        }
        upload_buffer.resource().unmap(0);

        gpu.immediate_pool.scope_immediate(|cmd| {
            upload_buffer.transition(cmd, d3d12::ResourceStates::COPY_SOURCE);
            resource.transition(cmd, d3d12::ResourceStates::COPY_DEST);

            cmd.copy_resource(upload_buffer.resource(), resource.resource());

            resource.transition(cmd, d3d12::ResourceStates::COMMON);

            Ok(())
        })?;

        Ok(VertexBuffer {
            resource,
            size: data.len() as u32,
            length: data.len() as u32 / stride,
            stride,
        })
    }

    pub fn bind_single(&self, cmd: &mut d3d12::GraphicsCommandList, slot: u32) {
        cmd.ia_set_vertex_buffers(
            slot,
            &[d3d12::VertexBufferView::new(
                self.resource.resource().gpu_virtual_address(),
                self.size,
                self.stride,
            )],
        );
    }

    // /// # Safety
    // ///
    // /// The caller must ensure that the data fits within the buffer.
    // pub unsafe fn write(&self, cmd: &DeviceContext, data: &[u8]) -> anyhow::Result<()> {
    //     unsafe {
    //         let m = cmd.map_unchecked(&self.buffer, 0, d3d11::MapType::WriteDiscard, false)?;
    //         m.data.copy_from(data.as_ptr() as _, data.len());
    //         cmd.unmap(&self.buffer, 0);

    //         Ok(())
    //     }
    // }
}
