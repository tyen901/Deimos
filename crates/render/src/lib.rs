use std::ffi::CStr;

#[macro_use]
extern crate tracing;

#[unsafe(no_mangle)]
pub static D3D12SDKVersion: u32 = 618;

#[unsafe(no_mangle)]
pub static D3D12SDKPath: &CStr = c".\\D3D12\\";

pub mod gpu;
pub mod tfx;
pub mod util;
