use deimos_data::tfx::ShaderStage;
use tiger_pkg::TagHash;

use crate::{
    asset::{Handle, index_buffer::IndexBuffer, vertex_buffer::VertexBuffer},
    gpu::{buffer::ImmutableBuffer, command_list::CommandList},
    renderer::Renderer,
};

pub(super) struct ModelBuffers {
    pub vertex0_buffer: Handle<VertexBuffer>,
    pub vertex1_buffer: Option<Handle<VertexBuffer>>,
    pub index_buffer: Handle<IndexBuffer>,
    pub color_buffer: ImmutableBuffer,
}

impl ModelBuffers {
    pub fn load(
        renderer: &Renderer,
        vertex0_buffer: TagHash,
        vertex1_buffer: TagHash,
        index_buffer: TagHash,
        color_buffer: TagHash,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            // vertex0_buffer: VertexBuffer::load(&renderer.gpu, vertex0_buffer)?,
            vertex0_buffer: renderer.asset_manager.load(vertex0_buffer),
            vertex1_buffer: if vertex1_buffer.is_some() {
                Some(renderer.asset_manager.load(vertex1_buffer))
            } else {
                None
            },
            index_buffer: renderer.asset_manager.load(index_buffer),
            color_buffer: if color_buffer.is_some() {
                let (vb_data, _) = VertexBuffer::get_raw_data_and_stride(color_buffer)
                    .expect("Failed to load color buffer for dynamic model");
                ImmutableBuffer::new(
                    &renderer.gpu,
                    &format!("color_buffer_{color_buffer}"),
                    d3d12::Format::R8g8b8a8Unorm,
                    &vb_data,
                )
                .expect("Failed to create color buffer for dynamic model")
            } else {
                ImmutableBuffer::new(
                    &renderer.gpu,
                    "color_buffer_fallback",
                    d3d12::Format::R8g8b8a8Unorm,
                    &[255u8, 255, 255, 255],
                )
                .expect("Failed to create color buffer for dynamic model")
            },
        })
    }

    #[profiling::function]
    pub fn bind(&self, cmd: &mut CommandList) -> Option<()> {
        self.index_buffer.get()?.bind(cmd);

        if let Some(vertex1) = &self.vertex1_buffer {
            cmd.ia_set_vertex_buffers(
                0,
                &[self.vertex0_buffer.get()?.view(), vertex1.get()?.view()],
            );
        } else {
            self.vertex0_buffer.get()?.bind_single(&cmd.cmd, 0);
        }

        self.color_buffer.bind_srv(cmd, ShaderStage::Vertex, 0);

        Some(())
    }

    pub fn is_loaded(&self) -> bool {
        self.index_buffer.is_loaded()
            && self.vertex0_buffer.is_loaded()
            && self.vertex1_buffer.as_ref().is_none_or(|v| v.is_loaded())
    }
}
