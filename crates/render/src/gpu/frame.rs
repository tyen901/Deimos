use std::sync::atomic::AtomicU64;

use d3d12::ext::GpuFence;

use crate::gpu::native_command_list::NativeCommandList;

/// Represents a frame in flight.
pub struct FrameContext {
    pub command_list: NativeCommandList,
    fence_value: AtomicU64,
}

impl FrameContext {
    pub fn new(device: &d3d12::Device) -> anyhow::Result<Self> {
        Ok(FrameContext {
            command_list: NativeCommandList::new(device)?,
            fence_value: AtomicU64::new(0),
        })
    }

    pub fn wait_for_completion(&self, fence: &GpuFence) -> anyhow::Result<()> {
        let fence_value = self.fence_value.load(std::sync::atomic::Ordering::Acquire);
        fence.wait(fence_value)?;
        Ok(())
    }

    pub fn signal(&self, fence: &GpuFence, queue: &d3d12::CommandQueue) {
        let fence_value = fence.signal(queue);
        self.fence_value
            .store(fence_value, std::sync::atomic::Ordering::Release);
    }
}
