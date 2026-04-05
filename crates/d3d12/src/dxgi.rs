use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory, IDXGIAdapter, IDXGIFactory};

use crate::Result;

#[repr(transparent)]
#[derive(Clone)]
pub struct Adapter(pub(crate) IDXGIAdapter);

impl Adapter {
    pub fn desc(&self) -> Result<AdapterDesc> {
        let desc = unsafe { self.0.GetDesc()? };

        Ok(AdapterDesc {
            description: String::from_utf16_lossy(&desc.Description)
                .trim_end_matches('\0')
                .trim()
                .to_string(),
            vendor_id: desc.VendorId,
            device_id: desc.DeviceId,
            sub_sys_id: desc.SubSysId,
            revision: desc.Revision,
            dedicated_video_memory: desc.DedicatedVideoMemory,
            dedicated_system_memory: desc.DedicatedSystemMemory,
            shared_system_memory: desc.SharedSystemMemory,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AdapterDesc {
    pub description: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub sub_sys_id: u32,
    pub revision: u32,
    pub dedicated_video_memory: usize,
    pub dedicated_system_memory: usize,
    pub shared_system_memory: usize,
}

pub struct AdapterIterator {
    factory: IDXGIFactory,
    index: u32,
}

impl AdapterIterator {
    pub fn new() -> Result<Self> {
        Ok(Self {
            factory: unsafe { CreateDXGIFactory()? },
            index: 0,
        })
    }
}

impl Iterator for AdapterIterator {
    type Item = Adapter;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let result = self.factory.EnumAdapters(self.index);
            self.index += 1;
            result.ok().map(Adapter)
        }
    }
}
