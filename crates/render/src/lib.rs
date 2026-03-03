use std::ffi::CStr;

#[macro_use]
extern crate tracing;

#[unsafe(no_mangle)]
#[cfg(debug_assertions)]
pub static D3D12SDKVersion: u32 = 619;

#[unsafe(no_mangle)]
#[cfg(debug_assertions)]
pub static D3D12SDKPath: &CStr = c".\\D3D12\\";

pub mod asset;
pub mod camera;
pub mod ecs;
pub mod features;
pub mod gpu;
pub mod renderer;
pub mod tfx;
pub mod util;
pub mod visibility;

mod temp_renderer;
