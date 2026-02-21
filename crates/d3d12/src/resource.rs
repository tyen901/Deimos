use std::mem::transmute;

use windows::Win32::Graphics::Direct3D12::ID3D12Resource;

use crate::GpuVirtualAddress;

#[repr(transparent)]
#[derive(Clone)]
pub struct Resource(pub(crate) ID3D12Resource);

impl Resource {
    pub fn gpu_virtual_address(&self) -> GpuVirtualAddress {
        GpuVirtualAddress(unsafe { self.0.GetGPUVirtualAddress() })
    }
}

impl AsRef<Resource> for ID3D12Resource {
    fn as_ref(&self) -> &Resource {
        unsafe { transmute(self) }
    }
}
