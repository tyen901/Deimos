use crate::gpu::command_list::CommandList;

/// Represents a frame in flight.
pub struct FrameContext {
    pub command_list: CommandList,
    fence: d3d12::ext::GpuFence,
}

impl FrameContext {
    pub fn new(device: &d3d12::Device) -> anyhow::Result<Self> {
        Ok(FrameContext {
            command_list: CommandList::new(device)?,
            fence: d3d12::ext::GpuFence::new(device)?,
        })
    }

    pub fn wait_for_completion(&self) -> anyhow::Result<()> {
        self.fence.wait()?;
        Ok(())
    }

    pub fn signal(&self, queue: &d3d12::CommandQueue) {
        self.fence.signal(queue);
    }
}
