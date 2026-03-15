use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use crate::{CommandQueue, Device, Error, Fence, Result, WaitResult, WaitableObject};

/// A fence wrapper that handles signaling and waiting
pub struct GpuFence {
    fence: Fence,
    event: WaitableObject,

    next_value: AtomicU64,
}

impl GpuFence {
    /// Creates a new `AutoFence`
    pub fn new(device: &Device) -> Result<Self> {
        let fence = device.create_fence(0)?;
        let event = WaitableObject::new(false, false)?;
        Ok(Self {
            fence,
            event,

            next_value: AtomicU64::new(1),
        })
    }

    /// Waits for the GPU to reach the previously set fence value
    pub fn wait(&self, value: u64, timeout: Option<Duration>) -> Result<()> {
        if self.fence.get_completed_value() >= value {
            return Ok(());
        }
        self.fence.set_event_on_completion(&self.event, value)?;
        if self.event.wait(timeout) == WaitResult::Timeout {
            return Err(Error::Other("Timed out waiting for fence".to_string()));
        }

        Ok(())
    }

    /// Queues a signal for the fence and returns the key. Calling await will block until the given signal key has been reached.
    pub fn signal(&self, queue: &CommandQueue) -> u64 {
        let value = self.next_value.fetch_add(1, Ordering::Relaxed);
        queue
            .signal(&self.fence, value)
            .expect("failed to signal fence on queue");
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
            value: AtomicU64::new(1),
        })
    }

    pub fn wait_for_previous(&self, timeout: Option<Duration>) -> Result<()> {
        self.fence.wait(self.value.load(Ordering::Relaxed), timeout)
    }

    pub fn wait_for_next(&self, timeout: Option<Duration>) -> Result<()> {
        self.fence
            .wait(self.value.load(Ordering::Relaxed) + 1, timeout)
    }

    pub fn signal(&self, queue: &CommandQueue) {
        let fence_value = self.fence.signal(queue);
        self.value.store(fence_value, Ordering::Relaxed);
    }
}
