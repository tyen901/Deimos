use bon::Builder;
use windows::Win32::Graphics::Direct3D12::*;

use crate::{verify_ffi_struct, Format};

#[repr(C)]
#[derive(Builder, Clone)]
pub struct RenderTargetViewDesc {
    pub format: Format,
    pub view_dimension: RtvDimension,
}
verify_ffi_struct!(RenderTargetViewDesc, D3D12_RENDER_TARGET_VIEW_DESC);

#[repr(i32)]
#[derive(Clone)]
pub enum RtvDimension {
    Buffer {
        first_element: u32,
        num_elements: u32,
    } = D3D12_RTV_DIMENSION_BUFFER.0,
    Texture1D {
        mip_slice: u32,
    } = D3D12_RTV_DIMENSION_TEXTURE1D.0,
    Texture1DArray {
        mip_slice: u32,
        first_array_element: u32,
        array_size: u32,
    } = D3D12_RTV_DIMENSION_TEXTURE1DARRAY.0,
    Texture2D {
        mip_slice: u32,
        plane_slice: u32,
    } = D3D12_RTV_DIMENSION_TEXTURE2D.0,
    Texture2DArray {
        mip_slice: u32,
        first_array_element: u32,
        array_size: u32,
        plane_slice: u32,
    } = D3D12_RTV_DIMENSION_TEXTURE2DARRAY.0,
    Texture2DMS = D3D12_RTV_DIMENSION_TEXTURE2DMS.0,
    Texture2DMSArray {
        first_array_slice: u32,
        array_size: u32,
    } = D3D12_RTV_DIMENSION_TEXTURE2DMSARRAY.0,
    Texture3D {
        mip_slice: u32,
        first_w_slice: u32,
        w_size: u32,
    } = D3D12_RTV_DIMENSION_TEXTURE3D.0,
}
