use std::sync::Arc;

use anyhow::Context;
use d3d12::Format;
use deimos_data::tfx::buffers::IndexBufferHeader;
use gpu_allocator::d3d12::{ResourceCreateDesc, ResourceType};
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use crate::gpu::{Gpu, alloc::resource::OwnedResource};

pub struct IndexBuffer {
    pub resource: OwnedResource,
    /// Amount of elements in the buffer
    pub length: usize,
    pub format: Format,
}

impl IndexBuffer {
    pub fn load(gpu: &Arc<Gpu>, hash: TagHash) -> anyhow::Result<Self> {
        let entry = package_manager()
            .get_entry(hash)
            .context("Entry not found")?;

        let header: IndexBufferHeader = package_manager()
            .read_tag_struct(hash)
            .context("Failed to read header data")?;
        let data = package_manager()
            .read_tag(entry.reference)
            .context("Failed to read buffer data")?;

        let desc = d3d12::ResourceDesc::buffer(data.len() as u64);
        let mut resource = gpu.allocate_resource(&ResourceCreateDesc {
            name: "index_buffer",
            memory_location: gpu_allocator::MemoryLocation::GpuOnly,
            resource_category: gpu_allocator::d3d12::ResourceCategory::Buffer,
            resource_desc: unsafe { &*desc.as_ffi() },
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

        Ok(IndexBuffer {
            resource,
            length: header.data_size as usize / if header.is_32bit { 4 } else { 2 },
            format: if header.is_32bit {
                Format::R32Uint
            } else {
                Format::R16Uint
            },
        })
    }

    // pub fn load_u16(gpu: &Gpu, data: &[u16]) -> anyhow::Result<Self> {
    //     let buffer = gpu.create_buffer(
    //         &BufferDesc::builder()
    //             .byte_width(std::mem::size_of_val(data) as u32)
    //             .usage(Usage::Immutable)
    //             .bind_flags(BindFlags::INDEX_BUFFER)
    //             .build(),
    //         Some(bytemuck::cast_slice(data)),
    //     )?;

    //     Ok(Self {
    //         buffer,
    //         length: data.len(),
    //         format: Format::R16Uint,
    //     })
    // }

    // pub fn load_u32(gpu: &Gpu, data: &[u32]) -> anyhow::Result<Self> {
    //     let buffer = gpu.create_buffer(
    //         &BufferDesc::builder()
    //             .byte_width(std::mem::size_of_val(data) as u32)
    //             .usage(Usage::Immutable)
    //             .bind_flags(BindFlags::INDEX_BUFFER)
    //             .build(),
    //         Some(bytemuck::cast_slice(data)),
    //     )?;

    //     Ok(Self {
    //         buffer,
    //         length: data.len(),
    //         format: Format::R32Uint,
    //     })
    // }

    pub fn bind(&self, cmd: &d3d12::GraphicsCommandList) {
        // cmd.input_assembler_set_index_buffer(&self.buffer, self.format, 0);
        cmd.ia_set_index_buffer(
            self.resource.resource().gpu_virtual_address(),
            self.length as u32,
            self.format,
        );
    }
}
