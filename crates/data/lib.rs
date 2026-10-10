#![forbid(unsafe_code)]

pub mod activity;
#[cfg(feature = "cpu")]
pub mod dxgi;
pub mod hash;
#[cfg(not(feature = "cpu"))]
pub mod investment;
pub mod map;
pub mod pattern;
#[cfg(not(feature = "cpu"))]
pub mod strings;
#[cfg(not(feature = "cpu"))]
pub mod tag;
#[cfg(feature = "cpu")]
#[path = "tag_cpu.rs"]
pub mod tag;
pub mod tfx;
pub mod umbra;
mod wide_hash;
#[cfg(not(feature = "cpu"))]
pub mod wwise;
