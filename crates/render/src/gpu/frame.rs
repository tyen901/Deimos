use std::sync::atomic::{AtomicU64, Ordering};

use d3d12::{CommandAllocator, Event, GraphicsCommandList};

/// Represents a frame in flight.
pub struct FrameContext {
    pub command_allocator: CommandAllocator,
    pub command_list: GraphicsCommandList,
    fence: d3d12::Fence,
    fence_event: d3d12::Event,

    value: AtomicU64,
    next_value: AtomicU64,
}

impl FrameContext {
    pub fn new(device: &d3d12::Device) -> anyhow::Result<Self> {
        let command_allocator = device.create_command_allocator(d3d12::CommandListType::Direct)?;
        let command_list = device.create_command_list(
            0,
            d3d12::CommandListType::Direct,
            &command_allocator,
            None,
        )?;
        command_list.close()?;
        Ok(FrameContext {
            command_allocator,
            command_list,
            fence: device.create_fence(0)?,
            fence_event: Event::new(false, false)?,
            value: AtomicU64::new(0),
            next_value: AtomicU64::new(1),
        })
    }

    pub fn wait_for_fence(&self) -> anyhow::Result<()> {
        let value = self.value.load(Ordering::Relaxed);
        if self.fence.get_completed_value() >= value {
            return Ok(());
        }
        self.fence
            .set_event_on_completion(&self.fence_event, value)?;
        self.fence_event.wait(None);
        Ok(())
    }

    pub fn signal(&self) {
        let value = self.next_value.fetch_add(1, Ordering::Relaxed);
        _ = self.fence.signal(value);
        self.value.store(value, Ordering::Relaxed);
    }
}
