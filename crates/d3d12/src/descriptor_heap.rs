use windows::Win32::Graphics::Direct3D12::*;

use crate::{CpuDescriptorHandle, GpuDescriptorHandle};

#[repr(transparent)]
#[derive(Clone)]
pub struct DescriptorHeap(pub(crate) ID3D12DescriptorHeap);

impl DescriptorHeap {
    pub fn cpu_descriptor_handle_for_heap_start(&self) -> CpuDescriptorHandle {
        CpuDescriptorHandle::from(unsafe { self.0.GetCPUDescriptorHandleForHeapStart() })
    }

    pub fn gpu_descriptor_handle_for_heap_start(&self) -> GpuDescriptorHandle {
        GpuDescriptorHandle::from(unsafe { self.0.GetGPUDescriptorHandleForHeapStart() })
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DescriptorHeapType {
    CbvSrvUav = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV.0,
    Sampler = D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER.0,
    Rtv = D3D12_DESCRIPTOR_HEAP_TYPE_RTV.0,
    Dsv = D3D12_DESCRIPTOR_HEAP_TYPE_DSV.0,
}
