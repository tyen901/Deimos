use tiger_pkg::TagHash;

use crate::{
    asset::{Handle, index_buffer::IndexBuffer, vertex_buffer::VertexBuffer},
    gpu::command_list::CommandList,
    renderer::Renderer,
};

pub(super) struct ModelBuffers {
    key: (TagHash, TagHash, TagHash),
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
            key: (vertex0_buffer, vertex1_buffer, index_buffer),
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
        if cmd.bound_modelbuffers == self.key {
            return Some(());
        }

        self.index_buffer.get_ref(|ib| ib.bind(cmd))?;

        if let Some(vertex1) = &self.vertex1_buffer {
            self.vertex0_buffer.get_ref(|v0| {
                vertex1.get_ref(|v1| {
                    cmd.ia_set_vertex_buffers(0, &[v0.view(), v1.view()]);
                })
            })?;
        } else {
            self.vertex0_buffer
                .get_ref(|vb| vb.bind_single(&cmd.cmd, 0))?;
        }

        cmd.bound_modelbuffers = self.key;

        Some(())
    }

    pub fn is_loaded(&self) -> bool {
        self.index_buffer.is_loaded()
            && self.vertex0_buffer.is_loaded()
            && self.vertex1_buffer.as_ref().map_or(true, |v| v.is_loaded())
    }
}
