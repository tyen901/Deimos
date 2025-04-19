use tiger_pkg::TagHash;

use crate::{
    asset::{index_buffer::IndexBuffer, vertex_buffer::VertexBuffer, Handle},
    gpu::command_list::CommandList,
    Renderer,
};

pub(super) struct ModelBuffers {
    pub vertex0_buffer: Handle<VertexBuffer>,
    pub vertex1_buffer: Option<Handle<VertexBuffer>>,
    pub index_buffer: Handle<IndexBuffer>,
}

impl ModelBuffers {
    pub fn load(vertex0_buffer: TagHash, vertex1_buffer: TagHash, index_buffer: TagHash) -> Self {
        let assets = &Renderer::instance().asset_manager;
        Self {
            vertex0_buffer: assets.load(vertex0_buffer),
            vertex1_buffer: assets.try_load(vertex1_buffer),
            index_buffer: assets.load(index_buffer),
        }
    }

    pub fn bind(&self, cmd: &mut CommandList) -> Option<()> {
        let vertex0 = self.vertex0_buffer.get()?;
        let index = self.index_buffer.get()?;

        index.bind(cmd);
        if let Some(vertex1) = &self.vertex1_buffer {
            let vertex1 = vertex1.get()?;
            cmd.input_assembler_set_vertex_buffers(
                0,
                &[Some(vertex0.buffer.clone()), Some(vertex1.buffer.clone())],
                Some(&[vertex0.stride as _, vertex1.stride as _]),
                Some(&[0, 0]),
            )
            .ok()?;
        } else {
            cmd.input_assembler_set_vertex_buffers(
                0,
                &[Some(vertex0.buffer.clone())],
                Some(&[vertex0.stride as _]),
                Some(&[0]),
            )
            .ok()?;
        }

        Some(())
    }
}
