use std::{
    marker::PhantomData,
    mem::{transmute, transmute_copy, ManuallyDrop},
};

use bitflags::bitflags;
use windows::Win32::Graphics::Direct3D12::*;

use crate::{impl_device_child, verify_ffi_type, Format, GpuVirtualAddress, Result, SampleDesc};

#[repr(transparent)]
#[derive(Clone)]
pub struct Resource(pub(crate) ID3D12Resource);
impl_device_child!(Resource);

impl Resource {
    pub fn gpu_virtual_address(&self) -> GpuVirtualAddress {
        GpuVirtualAddress(unsafe { self.0.GetGPUVirtualAddress() })
    }

    pub fn desc(&self) -> ResourceDesc {
        unsafe { transmute(self.0.GetDesc()) }
    }

    pub fn map(&self, subresource: u32) -> Result<*mut u8> {
        let mut data = std::ptr::null_mut();

        unsafe { self.0.Map(subresource, None, Some(&mut data)) }?;

        assert!(!data.is_null());

        Ok(data.cast())
    }

    pub fn unmap(&self, subresource: u32) {
        unsafe { self.0.Unmap(subresource, None) };
    }
}

impl AsRef<Resource> for ID3D12Resource {
    fn as_ref(&self) -> &Resource {
        unsafe { transmute(self) }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct ResourceDesc(pub(crate) D3D12_RESOURCE_DESC);
verify_ffi_type!(ResourceDesc, D3D12_RESOURCE_DESC);

impl ResourceDesc {
    pub fn new(dimension: ResourceDimension) -> Self {
        Self(D3D12_RESOURCE_DESC {
            Dimension: dimension.into(),
            Alignment: D3D12_DEFAULT_RESOURCE_PLACEMENT_ALIGNMENT as u64,
            Width: 1,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: Format::Unknown.into(),
            SampleDesc: SampleDesc::default().into(),
            Layout: TextureLayout::default().into(),
            Flags: ResourceFlags::default().into(),
        })
    }

    pub const fn alignment(mut self, alignment: u64) -> Self {
        self.0.Alignment = alignment;
        self
    }

    pub const fn width(mut self, width: u64) -> Self {
        self.0.Width = width;
        self
    }

    pub const fn height(mut self, height: u32) -> Self {
        self.0.Height = height;
        self
    }

    pub const fn depth_or_array_size(mut self, depth_or_array_size: u16) -> Self {
        self.0.DepthOrArraySize = depth_or_array_size;
        self
    }

    pub const fn mip_levels(mut self, mip_levels: u16) -> Self {
        self.0.MipLevels = mip_levels;
        self
    }

    pub fn format(mut self, format: Format) -> Self {
        self.0.Format = format.into();
        self
    }

    pub fn layout(mut self, layout: TextureLayout) -> Self {
        self.0.Layout = layout.into();
        self
    }

    pub fn flags(mut self, flags: ResourceFlags) -> Self {
        self.0.Flags = flags.into();
        self
    }
}

impl ResourceDesc {
    pub fn buffer(size: u64) -> Self {
        Self::new(ResourceDimension::Buffer)
            .width(size)
            .height(1)
            .format(Format::Unknown)
            .layout(TextureLayout::RowMajor)
    }
}

impl AsRef<D3D12_RESOURCE_DESC> for ResourceDesc {
    fn as_ref(&self) -> &D3D12_RESOURCE_DESC {
        &self.0
    }
}

#[repr(i32)]
#[derive(Default, Debug, Clone, Copy)]
pub enum TextureLayout {
    #[default]
    Unknown = D3D12_TEXTURE_LAYOUT_UNKNOWN.0,
    RowMajor = D3D12_TEXTURE_LAYOUT_ROW_MAJOR.0,
    _64KbUndefinedSwizzle = D3D12_TEXTURE_LAYOUT_64KB_UNDEFINED_SWIZZLE.0,
    _64KbStandardSwizzle = D3D12_TEXTURE_LAYOUT_64KB_STANDARD_SWIZZLE.0,
}
verify_ffi_type!(TextureLayout, D3D12_TEXTURE_LAYOUT);

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
verify_ffi_type!(ResourceFlags, D3D12_RESOURCE_FLAGS);

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum ResourceDimension {
    Buffer = D3D12_RESOURCE_DIMENSION_BUFFER.0,
    Texture1D = D3D12_RESOURCE_DIMENSION_TEXTURE1D.0,
    Texture2D = D3D12_RESOURCE_DIMENSION_TEXTURE2D.0,
    Texture3D = D3D12_RESOURCE_DIMENSION_TEXTURE3D.0,
}
verify_ffi_type!(ResourceDimension, D3D12_RESOURCE_DIMENSION);

#[derive(Debug, Clone)]
pub struct CopyableFootprints {
    /// One entry per subresource
    pub layouts: Vec<PlacedSubresourceFootprint>,
    /// One entry per subresource
    pub num_rows: Vec<u32>,
    /// One entry per subresource
    pub row_size_in_bytes: Vec<u64>,
    pub total_bytes: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PlacedSubresourceFootprint {
    pub offset: u64,
    pub footprint: SubresourceFootprint,
}
verify_ffi_type!(
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
verify_ffi_type!(SubresourceFootprint, D3D12_SUBRESOURCE_FOOTPRINT);

#[repr(transparent)]
#[derive(Clone)]
pub struct TextureCopyLocation<'a>(pub(crate) D3D12_TEXTURE_COPY_LOCATION, PhantomData<&'a ()>);

impl<'a> TextureCopyLocation<'a> {
    pub const fn subresource(resource: &'a Resource, subresource: u32) -> Self {
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
                    PlacedFootprint: unsafe {
                        transmute::<PlacedSubresourceFootprint, D3D12_PLACED_SUBRESOURCE_FOOTPRINT>(
                            footprint,
                        )
                    },
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

#[repr(transparent)]
#[derive(Clone)]
pub struct ResourceBarrier<'a>(pub(crate) D3D12_RESOURCE_BARRIER, PhantomData<&'a ()>);

impl<'a> ResourceBarrier<'a> {
    pub fn transition(
        resource: &'a Resource,
        subresource: u32,
        state_before: ResourceStates,
        state_after: ResourceStates,
    ) -> Self {
        Self(
            D3D12_RESOURCE_BARRIER {
                Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
                Flags: D3D12_RESOURCE_BARRIER_FLAGS(0),

                Anonymous: D3D12_RESOURCE_BARRIER_0 {
                    Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                        pResource: unsafe { transmute_copy(&resource.0) },
                        Subresource: subresource,
                        StateBefore: D3D12_RESOURCE_STATES(state_before.bits()),
                        StateAfter: D3D12_RESOURCE_STATES(state_after.bits()),
                    }),
                },
            },
            PhantomData,
        )
    }

    pub const fn aliasing(resource_before: &'a Resource, resource_after: &'a Resource) -> Self {
        Self(
            D3D12_RESOURCE_BARRIER {
                Type: D3D12_RESOURCE_BARRIER_TYPE_ALIASING,
                Flags: D3D12_RESOURCE_BARRIER_FLAGS(0),

                Anonymous: D3D12_RESOURCE_BARRIER_0 {
                    Aliasing: ManuallyDrop::new(D3D12_RESOURCE_ALIASING_BARRIER {
                        pResourceBefore: unsafe { transmute_copy(&resource_before.0) },
                        pResourceAfter: unsafe { transmute_copy(&resource_after.0) },
                    }),
                },
            },
            PhantomData,
        )
    }

    pub const fn uav(resource: &'a Resource) -> Self {
        Self(
            D3D12_RESOURCE_BARRIER {
                Type: D3D12_RESOURCE_BARRIER_TYPE_UAV,
                Flags: D3D12_RESOURCE_BARRIER_FLAGS(0),

                Anonymous: D3D12_RESOURCE_BARRIER_0 {
                    UAV: ManuallyDrop::new(D3D12_RESOURCE_UAV_BARRIER {
                        pResource: unsafe { transmute_copy(&resource.0) },
                    }),
                },
            },
            PhantomData,
        )
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ResourceStates : i32 {
        const COMMON = D3D12_RESOURCE_STATE_COMMON.0;
        const VERTEX_AND_CONSTANT_BUFFER = D3D12_RESOURCE_STATE_VERTEX_AND_CONSTANT_BUFFER.0;
        const INDEX_BUFFER = D3D12_RESOURCE_STATE_INDEX_BUFFER.0;
        const RENDER_TARGET = D3D12_RESOURCE_STATE_RENDER_TARGET.0;
        const UNORDERED_ACCESS = D3D12_RESOURCE_STATE_UNORDERED_ACCESS.0;
        const DEPTH_WRITE = D3D12_RESOURCE_STATE_DEPTH_WRITE.0;
        const DEPTH_READ = D3D12_RESOURCE_STATE_DEPTH_READ.0;
        const NON_PIXEL_SHADER_RESOURCE = D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE.0;
        const PIXEL_SHADER_RESOURCE = D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE.0;
        const STREAM_OUT = D3D12_RESOURCE_STATE_STREAM_OUT.0;
        const INDIRECT_ARGUMENT = D3D12_RESOURCE_STATE_INDIRECT_ARGUMENT.0;
        const COPY_DEST = D3D12_RESOURCE_STATE_COPY_DEST.0;
        const COPY_SOURCE = D3D12_RESOURCE_STATE_COPY_SOURCE.0;
        const RESOLVE_DEST = D3D12_RESOURCE_STATE_RESOLVE_DEST.0;
        const RESOLVE_SOURCE = D3D12_RESOURCE_STATE_RESOLVE_SOURCE.0;
        const RAYTRACING_ACCELERATION_STRUCTURE = D3D12_RESOURCE_STATE_RAYTRACING_ACCELERATION_STRUCTURE.0;
        const SHADING_RATE_SOURCE = D3D12_RESOURCE_STATE_SHADING_RATE_SOURCE.0;
        const RESERVED_INTERNAL_8000 = D3D12_RESOURCE_STATE_RESERVED_INTERNAL_8000.0;
        const RESERVED_INTERNAL_4000 = D3D12_RESOURCE_STATE_RESERVED_INTERNAL_4000.0;
        const RESERVED_INTERNAL_100000 = D3D12_RESOURCE_STATE_RESERVED_INTERNAL_100000.0;
        const RESERVED_INTERNAL_40000000 = D3D12_RESOURCE_STATE_RESERVED_INTERNAL_40000000.0;
        const RESERVED_INTERNAL_80000000 = D3D12_RESOURCE_STATE_RESERVED_INTERNAL_80000000.0;
        const GENERIC_READ = D3D12_RESOURCE_STATE_GENERIC_READ.0;
        const ALL_SHADER_RESOURCE = D3D12_RESOURCE_STATE_ALL_SHADER_RESOURCE.0;
        const PRESENT = D3D12_RESOURCE_STATE_PRESENT.0;
        const PREDICATION = D3D12_RESOURCE_STATE_PREDICATION.0;
        const VIDEO_DECODE_READ = D3D12_RESOURCE_STATE_VIDEO_DECODE_READ.0;
        const VIDEO_DECODE_WRITE = D3D12_RESOURCE_STATE_VIDEO_DECODE_WRITE.0;
        const VIDEO_PROCESS_READ = D3D12_RESOURCE_STATE_VIDEO_PROCESS_READ.0;
        const VIDEO_PROCESS_WRITE = D3D12_RESOURCE_STATE_VIDEO_PROCESS_WRITE.0;
        const VIDEO_ENCODE_READ = D3D12_RESOURCE_STATE_VIDEO_ENCODE_READ.0;
        const VIDEO_ENCODE_WRITE = D3D12_RESOURCE_STATE_VIDEO_ENCODE_WRITE.0;
    }
}
verify_ffi_type!(ResourceStates, D3D12_RESOURCE_STATES);
