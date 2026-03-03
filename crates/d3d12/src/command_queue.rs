use std::mem::transmute;

use bitflags::bitflags;
use bon::Builder;
use static_assertions::assert_eq_size;
use windows::Win32::Graphics::Direct3D12::*;

use crate::{Fence, GraphicsCommandList, Result};

#[repr(transparent)]
#[derive(Clone)]
pub struct CommandQueue(pub(crate) ID3D12CommandQueue);

impl CommandQueue {
    pub fn execute_command_lists(&self, command_lists: &[GraphicsCommandList]) {
        unsafe {
            self.0.ExecuteCommandLists(transmute::<
                &[GraphicsCommandList],
                &[Option<ID3D12CommandList>],
            >(command_lists));
        }
    }

    pub fn signal(&self, fence: &Fence, value: u64) -> Result<()> {
        unsafe {
            self.0.Signal(&fence.0, value)?;
        }
        Ok(())
    }

    pub fn wait(&self, fence: &Fence, value: u64) -> Result<()> {
        unsafe {
            self.0.Wait(&fence.0, value)?;
        }
        Ok(())
    }

    pub fn get_clock_calibration(&self) -> Result<ClockCalibration> {
        let mut calibration = ClockCalibration::default();
        unsafe {
            self.0.GetClockCalibration(
                &mut calibration.gpu_timestamp,
                &mut calibration.cpu_timestamp,
            )?;
        }
        Ok(calibration)
    }

    pub fn get_timestamp_frequency(&self) -> Result<u64> {
        Ok(unsafe { self.0.GetTimestampFrequency()? })
    }
}

#[repr(C)]
#[derive(Clone, Debug, Builder)]
pub struct CommandQueueDesc {
    pub type_: CommandListType,
    #[builder(default)]
    pub priority: i32,
    #[builder(default)]
    pub flags: CommandQueueFlags,
    #[builder(default)]
    pub node_mask: u32,
}
assert_eq_size!(CommandQueueDesc, D3D12_COMMAND_QUEUE_DESC);

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandListType {
    Bundle = D3D12_COMMAND_LIST_TYPE_BUNDLE.0,
    Compute = D3D12_COMMAND_LIST_TYPE_COMPUTE.0,
    Copy = D3D12_COMMAND_LIST_TYPE_COPY.0,
    Direct = D3D12_COMMAND_LIST_TYPE_DIRECT.0,
    None = D3D12_COMMAND_LIST_TYPE_NONE.0,
    VideoDecode = D3D12_COMMAND_LIST_TYPE_VIDEO_DECODE.0,
    VideoEncode = D3D12_COMMAND_LIST_TYPE_VIDEO_ENCODE.0,
    VideoProcess = D3D12_COMMAND_LIST_TYPE_VIDEO_PROCESS.0,
}

bitflags! {
    #[repr(transparent)]
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CommandQueueFlags : i32 {
        const DISABLE_GPU_TIMEOUT = D3D12_COMMAND_QUEUE_FLAG_DISABLE_GPU_TIMEOUT.0;
    }
}

#[derive(Default)]
pub struct ClockCalibration {
    pub gpu_timestamp: u64,
    pub cpu_timestamp: u64,
}
