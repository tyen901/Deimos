use std::sync::atomic::{AtomicU64, Ordering};

use crate::{CommandQueue, Device, Event, Fence, Result};

/// A fence wrapper that handles signaling and waiting
pub struct GpuFence {
    fence: Fence,
    event: Event,

    next_value: AtomicU64,
}

impl GpuFence {
    /// Creates a new `AutoFence`
    pub fn new(device: &Device) -> Result<Self> {
        let fence = device.create_fence(0)?;
        let event = Event::new(false, false)?;
        Ok(Self {
            fence,
            event,

            next_value: AtomicU64::new(1),
        })
    }

    /// Waits for the GPU to reach the previously set fence value
    pub fn wait(&self, value: u64) -> Result<()> {
        if self.fence.get_completed_value() >= value {
            return Ok(());
        }
        self.fence.set_event_on_completion(&self.event, value)?;
        self.event.wait(None);
        Ok(())
    }

    /// Queues a signal for the fence and returns the key. Calling await will block until the given signal key has been reached.
    pub fn signal(&self, queue: &CommandQueue) -> u64 {
        let value = self.next_value.fetch_add(1, Ordering::Relaxed);
        _ = queue.signal(&self.fence, value);
        value
    }
}

/// An even simpler GPU fence that tracks the last signal value and waits for it to be reached.
pub struct GpuFenceWaiter {
    fence: GpuFence,
    value: AtomicU64,
}

impl GpuFenceWaiter {
    pub fn new(device: &Device) -> Result<Self> {
        Ok(Self {
            fence: GpuFence::new(device)?,
            value: AtomicU64::new(0),
        })
    }

    pub fn wait(&self) -> Result<()> {
        self.fence.wait(self.value.load(Ordering::Relaxed))
    }

    pub fn signal(&self, queue: &CommandQueue) {
        let fence_value = self.fence.signal(queue);
        self.value.store(fence_value, Ordering::Relaxed);
    }
}
