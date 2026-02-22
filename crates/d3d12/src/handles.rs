//! Newtypes for various handles/virtual addresses used throughout the API

use windows::Win32::Graphics::Direct3D12::{
    D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_GPU_DESCRIPTOR_HANDLE,
};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuVirtualAddress(pub(crate) u64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CpuDescriptorHandle(usize);

impl CpuDescriptorHandle {
    pub fn offset(&self, offset_in_descriptors: usize, increment_size: u32) -> Self {
        Self(self.0 + offset_in_descriptors * increment_size as usize)
    }

    pub fn index(&self, base: CpuDescriptorHandle, increment_size: u32) -> usize {
        self.0 - base.0 / increment_size as usize
    }
}

impl From<CpuDescriptorHandle> for D3D12_CPU_DESCRIPTOR_HANDLE {
    fn from(handle: CpuDescriptorHandle) -> Self {
        D3D12_CPU_DESCRIPTOR_HANDLE { ptr: handle.0 }
    }
}

impl From<D3D12_CPU_DESCRIPTOR_HANDLE> for CpuDescriptorHandle {
    fn from(handle: D3D12_CPU_DESCRIPTOR_HANDLE) -> Self {
        Self(handle.ptr)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpuDescriptorHandle(u64);

impl GpuDescriptorHandle {
    pub fn offset(&self, offset_in_descriptors: usize, increment_size: u32) -> Self {
        Self(self.0 + offset_in_descriptors as u64 * increment_size as u64)
    }

    pub fn index(&self, base: GpuDescriptorHandle, increment_size: u32) -> u64 {
        self.0 - base.0 / increment_size as u64
    }
}

impl From<GpuDescriptorHandle> for D3D12_GPU_DESCRIPTOR_HANDLE {
    fn from(handle: GpuDescriptorHandle) -> Self {
        D3D12_GPU_DESCRIPTOR_HANDLE { ptr: handle.0 }
    }
}

impl From<D3D12_GPU_DESCRIPTOR_HANDLE> for GpuDescriptorHandle {
    fn from(handle: D3D12_GPU_DESCRIPTOR_HANDLE) -> Self {
        Self(handle.ptr)
    }
}
