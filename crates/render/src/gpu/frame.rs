use std::{sync::atomic::AtomicU64, time::Duration};

use d3d12::ext::GpuFence;

use crate::gpu::{
    alloc::{descriptors::DescriptorRing, ring::UploadRing},
    profiler::FrameProfiler,
    stream::FrameCommandStream,
};

/// Represents a frame in flight.
pub struct FrameContext {
    pub stream: FrameCommandStream,
    fence_value: AtomicU64,

    pub upload: UploadRing,
    pub descriptors: DescriptorRing,
    pub profiler: FrameProfiler,
}

impl FrameContext {
    pub fn new(
        device: &d3d12::Device,
        allocator: &mut gpu_allocator::d3d12::Allocator,
    ) -> anyhow::Result<Self> {
        let mb = 1024 * 1024;
        Ok(Self {
            upload: UploadRing::new(device, 32 * mb)?,
            stream: FrameCommandStream::new(device.clone()),
            // command_list: NativeCommandList::new(device, d3d12::CommandListType::Direct)?,
            fence_value: AtomicU64::new(0),
            descriptors: DescriptorRing::new(
                device,
                d3d12::DescriptorHeapType::CbvSrvUav,
                512_000,
                true,
            )?,
            profiler: FrameProfiler::new(device, allocator)?,
        })
    }

    /// Resets upload rings.
    ///
    /// Keep in mind that this does not reset the command list.
    pub fn begin_frame(&self, queue: &d3d12::CommandQueue) {
        self.upload.reset();
        self.descriptors.reset();

        self.profiler.begin_frame(queue);
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
