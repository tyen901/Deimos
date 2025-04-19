use anyhow::Context;
use d3d11::{dxgi, BindFlags, BufferDesc, DeviceChild, ShaderResourceViewDesc};
use deimos_data::tfx::buffers::VertexBufferHeader;
use tiger_parse::PackageManagerExt;
use tiger_pkg::package_manager;
use tiger_pkg::TagHash;

use crate::{gpu::command_list::CommandList, Gpu};

#[derive(Clone)]
pub struct VertexBuffer {
    pub buffer: d3d11::Buffer,
    pub size: u32,
    pub length: u32,
    pub stride: u32,
    /// Optional SRV for the buffer. Created for buffers with stride 1 and 4
    pub srv: Option<d3d11::ShaderResourceView>,
}

impl VertexBuffer {
    #[profiling::function]
    pub fn load_data(device: &d3d11::Device, data: &[u8], stride: u32) -> anyhow::Result<Self> {
        let bind_flags = if matches!(stride, 1 | 4) {
            BindFlags::VERTEX_BUFFER | BindFlags::SHADER_RESOURCE
        } else {
            BindFlags::VERTEX_BUFFER
        };
        let buffer = device.create_buffer(
            &BufferDesc::builder()
                .byte_width(data.len() as u32)
                .usage(d3d11::Usage::Default)
                .bind_flags(bind_flags)
                .build(),
            Some(data),
        )?;

        let srv = if matches!(stride, 1 | 4) {
            Some(
                device.create_shader_resource_view(
                    &buffer,
                    Some(
                        &ShaderResourceViewDesc::builder()
                            .format(if stride == 1 {
                                dxgi::Format::R8Unorm
                            } else {
                                dxgi::Format::R8g8b8a8Unorm
                            })
                            .view_dimension(d3d11::srv::SrvDimension::Buffer {
                                first_element_or_element_offset: 0,
                                num_elements_or_element_width: (data.len() / stride as usize)
                                    as u32,
                            })
                            .build(),
                    ),
                )?,
            )
        } else {
            None
        };

        Ok(VertexBuffer {
            buffer,
            size: data.len() as u32,
            length: data.len() as u32 / stride,
            stride,
            srv,
        })
    }

    pub fn bind_single(&self, cmd: &mut CommandList, slot: u32) {
        cmd.input_assembler_set_vertex_buffers(
            slot,
            &[Some(self.buffer.clone())],
            Some(&[self.stride]),
            Some(&[0]),
        )
        .expect("Failed to bind vertex buffer");
    }
}

pub(crate) fn load_vertex_buffer(gctx: &Gpu, hash: TagHash) -> anyhow::Result<VertexBuffer> {
    let entry = package_manager()
        .get_entry(hash)
        .context("Entry not found")?;

    let header: VertexBufferHeader = package_manager()
        .read_tag_struct(hash)
        .context("Failed to read header data")?;
    let data = package_manager()
        .read_tag(entry.reference)
        .context("Failed to read buffer data")?;

    let vb = VertexBuffer::load_data(&gctx.device, &data, header.stride as _)?;
    vb.buffer.set_debug_name(format!("VertexBuffer {hash}"));
    Ok(vb)
}
