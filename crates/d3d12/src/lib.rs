pub use windows::Win32::Graphics::Direct3D12::*;

pub mod error;
pub use error::{Error, Result};

mod format;
pub use format::Format;
mod swapchain;
pub use swapchain::*;

mod blend;
pub use blend::*;

mod command_list;
pub use command_list::*;

mod command_queue;
pub use command_queue::*;

mod descriptor_heap;
pub use descriptor_heap::*;

mod device;
pub use device::*;

mod device_child;
pub use device_child::*;

mod dred;
pub use dred::*;

mod dsv;
pub use dsv::*;

mod heap;
pub use heap::*;

pub(crate) mod pix;

mod rasterizer;
pub use rasterizer::*;

mod pipeline;
pub use pipeline::*;

mod root_signature;
pub use root_signature::*;

mod handles;
pub use handles::*;

mod query;
pub use query::*;

mod sampler;
pub use sampler::*;

mod input_layout;
pub use input_layout::*;

mod fence;
pub use fence::*;

mod resource;
pub use resource::*;

mod reflection;
pub use reflection::*;

mod rtv;
pub use rtv::*;

mod srv;
pub use srv::*;

mod util;

pub mod ext;

pub const fn calc_subresource(
    mip_slice: u32,
    array_slice: u32,
    plane_slice: u32,
    mip_levels: u32,
    array_size: u32,
) -> u32 {
    mip_slice + array_slice * mip_levels + plane_slice * mip_levels * array_size
}
