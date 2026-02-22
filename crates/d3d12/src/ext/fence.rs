use std::sync::atomic::{AtomicU64, Ordering};

use crate::{CommandQueue, Device, Event, Fence, Result};

/// A fence wrapper that handles signaling and waiting
pub struct GpuFence {
    fence: Fence,
    event: Event,

    value: AtomicU64,
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

            value: AtomicU64::new(0),
            next_value: AtomicU64::new(1),
        })
    }

    /// Waits for the GPU to reach the previously set fence value
    pub fn wait(&self) -> Result<()> {
        let value = self.value.load(Ordering::Relaxed);
        if self.fence.get_completed_value() >= value {
            return Ok(());
        }
        self.fence.set_event_on_completion(&self.event, value)?;
        self.event.wait(None);
        Ok(())
    }

    /// Queues a signal for the fence. Calling await will block until the signal has been reached.
    pub fn signal(&self, queue: &CommandQueue) {
        let value = self.next_value.fetch_add(1, Ordering::Relaxed);
        _ = queue.signal(&self.fence, value);
        self.value.store(value, Ordering::Relaxed);
    }
}
