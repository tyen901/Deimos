use tracing::{error, info};
use windows::{
    core::{IUnknown, Interface},
    Win32::Graphics::{
        Direct3D12::{
            CLSID_D3D12SDKConfiguration, D3D12CreateDevice, D3D12GetInterface, ID3D12Device,
            ID3D12DeviceFactory, ID3D12SDKConfiguration1,
        },
        Dxgi::IDXGIAdapter,
    },
};

use crate::Device;

pub struct AgilitySdk {
    pub version: u32,
    pub path: String,
}

// https://github.com/gfx-rs/wgpu/blob/trunk/wgpu-hal/src/dx12/device_creation.rs
pub enum DeviceFactory {
    Independent(ID3D12DeviceFactory),
    Legacy,
}

impl DeviceFactory {
    pub fn new(agility_sdk: Option<AgilitySdk>) -> crate::Result<Self> {
        let Some(agility_sdk) = agility_sdk else {
            info!("No D3D12 Agility SDK configuration provided; using system D3D12 runtime");
            return Ok(Self::Legacy);
        };

        match Self::try_create_independent(&agility_sdk) {
            Ok(factory) => {
                info!("Successfully created D3D12 device factory with Agility SDK version {} at path '{}'", agility_sdk.version, agility_sdk.path);
                Ok(Self::Independent(factory))
            }
            Err(e) => {
                error!("Failed to create D3D12 device factory with Agility SDK: {e}; falling back to system D3D12 runtime");
                Ok(Self::Legacy)
            }
        }
    }

    fn try_create_independent(agility_sdk: &AgilitySdk) -> crate::Result<ID3D12DeviceFactory> {
        let mut sdk_config: Option<ID3D12SDKConfiguration1> = None;
        unsafe { D3D12GetInterface(&CLSID_D3D12SDKConfiguration, &mut sdk_config)? };
        let sdk_config = sdk_config.ok_or(crate::Error::InterfaceUnsupported)?;

        let sdk_path = std::ffi::CString::new(agility_sdk.path.as_bytes())
            .map_err(|_e| crate::Error::InvalidInput(agility_sdk.path.clone()))?;
        let factory: ID3D12DeviceFactory = unsafe {
            sdk_config.CreateDeviceFactory(
                agility_sdk.version,
                windows::core::PCSTR(sdk_path.as_ptr().cast::<u8>()),
            )
        }?;

        Ok(factory)
    }

    pub fn create_device(
        &self,
        adapter: &IDXGIAdapter,
        feature_level: windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL,
    ) -> crate::Result<Device> {
        match self {
            Self::Independent(factory) => {
                let mut device: Option<ID3D12Device> = None;
                unsafe {
                    factory.CreateDevice(
                        &adapter.cast::<IUnknown>().unwrap(),
                        feature_level,
                        &mut device,
                    )?;
                };

                Ok(Device(device.expect("created device should not be null")))
            }
            Self::Legacy => {
                let mut device = None;
                unsafe {
                    D3D12CreateDevice(
                        &adapter.cast::<IUnknown>().unwrap(),
                        feature_level,
                        &mut device,
                    )?;
                }

                Ok(Device(device.expect("created device should not be null")))
            }
        }
    }
}
