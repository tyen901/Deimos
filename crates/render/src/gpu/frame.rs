use std::{sync::atomic::AtomicU64, time::Duration};

use d3d12::ext::GpuFence;

use crate::gpu::{
    alloc::ring::UploadRing,
    native_command_list::{CommandListRing, NativeCommandList},
};

/// Represents a frame in flight.
pub struct FrameContext {
    pub command_list: NativeCommandList,
    fence_value: AtomicU64,

    pub upload: UploadRing,
}

impl FrameContext {
    pub fn new(device: &d3d12::Device) -> anyhow::Result<Self> {
        let mb = 1024 * 1024;
        Ok(FrameContext {
            upload: UploadRing::new(device, 64 * mb)?,
            command_list: NativeCommandList::new(device)?,
            fence_value: AtomicU64::new(0),
        })
    }

    /// Resets upload rings.
    ///
    /// Keep in mind that this does not reset the command list.
    pub fn begin_frame(&self) {
        self.upload.reset();
    }

    pub fn wait_for_completion(&self, fence: &GpuFence) -> anyhow::Result<()> {
        let fence_value = self.fence_value.load(std::sync::atomic::Ordering::Acquire);
        fence.wait(fence_value, Some(Duration::from_secs(5)))?;
        Ok(())
    }

    pub fn signal(&self, fence: &GpuFence, queue: &d3d12::CommandQueue) {
        let fence_value = fence.signal(queue);
        self.fence_value
            .store(fence_value, std::sync::atomic::Ordering::Release);
    }
}
