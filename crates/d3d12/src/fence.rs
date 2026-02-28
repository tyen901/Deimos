use std::time::Duration;

use bitflags::bitflags;
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE, WAIT_TIMEOUT},
    Graphics::Direct3D12::*,
    System::Threading::{CreateEventA, ResetEvent, WaitForSingleObject, INFINITE},
};

use crate::Result;

#[repr(transparent)]
#[derive(Clone)]
pub struct Fence(pub(crate) ID3D12Fence);

impl Fence {
    pub fn signal(&self, value: u64) -> Result<()> {
        unsafe { self.0.Signal(value)? };
        Ok(())
    }

    pub fn get_completed_value(&self) -> u64 {
        unsafe { self.0.GetCompletedValue() }
    }

    pub fn set_event_on_completion(&self, event: &Event, value: u64) -> Result<()> {
        unsafe { self.0.SetEventOnCompletion(value, event.0)? };
        Ok(())
    }
}

#[repr(transparent)]
pub struct Event(pub(crate) HANDLE);

impl Event {
    pub fn new(manual_reset: bool, initial_state: bool) -> Result<Self> {
        let event = unsafe { CreateEventA(None, manual_reset, initial_state, None)? };
        Ok(Self(event))
    }

    /// Wait for the event to be signaled.
    ///
    /// If `timeout` is `None`, the function will wait indefinitely.
    pub fn wait(&self, timeout: Option<Duration>) -> WaitResult {
        let res = unsafe {
            WaitForSingleObject(
                self.0,
                timeout.map(|d| d.as_millis() as u32).unwrap_or(INFINITE),
            )
        };

        match res {
            WAIT_TIMEOUT => WaitResult::Timeout,
            _ => WaitResult::Success,
        }
    }

    pub fn reset(&self) {
        _ = unsafe { ResetEvent(self.0) };
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        _ = unsafe { CloseHandle(self.0) };
    }
}

bitflags! {
    pub struct FenceFlags : i32 {
        const SHARED = D3D12_FENCE_FLAG_SHARED.0;
        const SHARED_CROSS_ADAPTER = D3D12_FENCE_FLAG_SHARED_CROSS_ADAPTER.0;
        const NON_MONITORED = D3D12_FENCE_FLAG_NON_MONITORED.0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitResult {
    Success,
    Timeout,
}
