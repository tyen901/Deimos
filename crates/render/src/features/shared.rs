use tiger_pkg::TagHash;

use crate::{
    asset::{Handle, index_buffer::IndexBuffer, vertex_buffer::VertexBuffer},
    gpu::command_list::CommandList,
    renderer::Renderer,
};

pub(super) struct ModelBuffers {
    pub vertex0_buffer: Handle<VertexBuffer>,
    pub vertex1_buffer: Option<Handle<VertexBuffer>>,
    pub index_buffer: Handle<IndexBuffer>,
}

impl ModelBuffers {
    pub fn load(
        renderer: &Renderer,
        vertex0_buffer: TagHash,
        vertex1_buffer: TagHash,
        index_buffer: TagHash,
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

        Some(())
    }
}
