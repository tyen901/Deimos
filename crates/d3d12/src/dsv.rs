use bon::Builder;
use windows::Win32::Graphics::Direct3D12::*;

use crate::{verify_ffi_type, Format};

#[repr(transparent)]
#[derive(Clone)]
pub struct DepthStencilViewDesc(pub(crate) D3D12_DEPTH_STENCIL_VIEW_DESC);
verify_ffi_type!(DepthStencilViewDesc, D3D12_DEPTH_STENCIL_VIEW_DESC);

impl DepthStencilViewDesc {
    pub fn texture_2d(format: Format) -> Self {
        Self(D3D12_DEPTH_STENCIL_VIEW_DESC {
            Format: format.into(),
            ViewDimension: D3D12_DSV_DIMENSION_TEXTURE2D,
            Anonymous: D3D12_DEPTH_STENCIL_VIEW_DESC_0 {
                Texture2D: D3D12_TEX2D_DSV { MipSlice: 0 },
            },
            Flags: D3D12_DSV_FLAGS(0),
        })
    }
}
