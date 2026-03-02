use windows::Win32::Graphics::{
    Direct3D::WKPDID_D3DDebugObjectName, Direct3D12::ID3D12DeviceChild,
};

use crate::device::Device;

pub trait DeviceChild {
    fn as_device_child(&self) -> &ID3D12DeviceChild;

    fn get_device(&self) -> Device {
        unsafe {
            let mut device = None;

            _ = self.as_device_child().GetDevice(&mut device);
            Device(device.unwrap())
        }
    }

    fn set_debug_name(&self, name: impl AsRef<str>) {
        let name_cstr = std::ffi::CString::new(name.as_ref()).unwrap();
        unsafe {
            let _ = self.as_device_child().SetPrivateData(
                &WKPDID_D3DDebugObjectName,
                name_cstr.to_bytes().len() as _,
                Some(name_cstr.as_ptr().cast()),
            );
        }
    }
}

#[macro_export]
macro_rules! impl_device_child {
    ($name:ident) => {
        impl $crate::device_child::DeviceChild for $name {
            fn as_device_child(&self) -> &windows::Win32::Graphics::Direct3D12::ID3D12DeviceChild {
                &self.0
            }
        }
    };
}
