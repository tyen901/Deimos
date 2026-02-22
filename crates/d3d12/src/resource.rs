use std::{
    marker::PhantomData,
    mem::{transmute, transmute_copy},
};

use bitflags::bitflags;
use bon::Builder;
use windows::Win32::Graphics::Direct3D12::*;

use crate::{verify_ffi_struct, Format, GpuVirtualAddress, SampleDesc};

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

#[repr(C)]
#[derive(Debug, Clone, Builder)]
pub struct ResourceDesc {
    #[builder(start_fn)]
    pub dimension: ResourceDimension,
    #[builder(default = D3D12_DEFAULT_RESOURCE_PLACEMENT_ALIGNMENT as u64)]
    pub alignment: u64,
    pub width: u64,
    pub height: u32,
    #[builder(default = 1)]
    pub depth_or_array_size: u16,
    #[builder(default = 1)]
    pub mip_levels: u16,
    pub format: Format,
    #[builder(default)]
    pub sample_desc: SampleDesc,
    #[builder(default)]
    pub layout: TextureLayout,
    #[builder(default)]
    pub flags: ResourceFlags,
}
verify_ffi_struct!(ResourceDesc, D3D12_RESOURCE_DESC);

#[repr(i32)]
#[derive(Default, Debug, Clone, Copy)]
pub enum TextureLayout {
    #[default]
    Unknown = D3D12_TEXTURE_LAYOUT_UNKNOWN.0,
    RowMajor = D3D12_TEXTURE_LAYOUT_ROW_MAJOR.0,
    _64KbUndefinedSwizzle = D3D12_TEXTURE_LAYOUT_64KB_UNDEFINED_SWIZZLE.0,
    _64KbStandardSwizzle = D3D12_TEXTURE_LAYOUT_64KB_STANDARD_SWIZZLE.0,
}

bitflags! {
    #[derive(Default, Debug, Clone, Copy)]
    pub struct ResourceFlags : i32 {
        const ALLOW_RENDER_TARGET = D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET.0;
        const ALLOW_DEPTH_STENCIL = D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL.0;
        const ALLOW_UNORDERED_ACCESS = D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS.0;
        const DENY_SHADER_RESOURCE = D3D12_RESOURCE_FLAG_DENY_SHADER_RESOURCE.0;
        const ALLOW_CROSS_ADAPTER = D3D12_RESOURCE_FLAG_ALLOW_CROSS_ADAPTER.0;
        const ALLOW_SIMULTANEOUS_ACCESS = D3D12_RESOURCE_FLAG_ALLOW_SIMULTANEOUS_ACCESS.0;
        const VIDEO_DECODE_REFERENCE_ONLY = D3D12_RESOURCE_FLAG_VIDEO_DECODE_REFERENCE_ONLY.0;
        const VIDEO_ENCODE_REFERENCE_ONLY = D3D12_RESOURCE_FLAG_VIDEO_ENCODE_REFERENCE_ONLY.0;
        const RAYTRACING_ACCELERATION_STRUCTURE = D3D12_RESOURCE_FLAG_RAYTRACING_ACCELERATION_STRUCTURE.0;
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum ResourceDimension {
    Buffer = D3D12_RESOURCE_DIMENSION_BUFFER.0,
    Texture1D = D3D12_RESOURCE_DIMENSION_TEXTURE1D.0,
    Texture2D = D3D12_RESOURCE_DIMENSION_TEXTURE2D.0,
    Texture3D = D3D12_RESOURCE_DIMENSION_TEXTURE3D.0,
}

#[derive(Debug, Clone)]
pub struct CopyableFootprints {
    pub layouts: Vec<PlacedSubresourceFootprint>,
    pub num_rows: u32,
    pub row_size_in_bytes: u64,
    pub total_bytes: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PlacedSubresourceFootprint {
    pub offset: u64,
    pub footprint: SubresourceFootprint,
}
verify_ffi_struct!(
    PlacedSubresourceFootprint,
    D3D12_PLACED_SUBRESOURCE_FOOTPRINT
);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SubresourceFootprint {
    pub format: Format,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub row_pitch: u32,
}
verify_ffi_struct!(SubresourceFootprint, D3D12_SUBRESOURCE_FOOTPRINT);

#[repr(transparent)]
#[derive(Clone)]
pub struct TextureCopyLocation<'a>(pub(crate) D3D12_TEXTURE_COPY_LOCATION, PhantomData<&'a ()>);

impl<'a> TextureCopyLocation<'a> {
    pub fn subresource(resource: &'a Resource, subresource: u32) -> Self {
        Self(
            D3D12_TEXTURE_COPY_LOCATION {
                pResource: unsafe { transmute_copy(&resource.0) },
                Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    SubresourceIndex: subresource,
                },
            },
            PhantomData,
        )
    }

    pub fn placed_footprint(resource: &'a Resource, footprint: PlacedSubresourceFootprint) -> Self {
        Self(
            D3D12_TEXTURE_COPY_LOCATION {
                pResource: unsafe { transmute_copy(&resource.0) },
                Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
                Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
                    PlacedFootprint: unsafe { transmute(footprint) },
                },
            },
            PhantomData,
        )
    }
}

// #[repr(i32)]
// #[derive(Debug, Clone, Copy)]
// pub enum TextureCopy {
//     SubresourceIndex(u32) = D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX.0,
//     PlacedFootprint(PlacedSubresourceFootprint) = D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT.0,
// }
