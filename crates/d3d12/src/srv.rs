use std::ops::Range;

use windows::Win32::Graphics::Direct3D12::*;

use crate::{Format, GpuVirtualAddress};

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ShaderResourceViewDesc(pub(crate) D3D12_SHADER_RESOURCE_VIEW_DESC);

impl ShaderResourceViewDesc {
    #[inline]
    pub fn buffer(
        format: Format,
        elements: Range<u64>,
        structure_byte_stride: u32,
        flags: u32,
    ) -> Self {
        Self(D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format: format.into(),
            ViewDimension: D3D12_SRV_DIMENSION_BUFFER,
            Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                Buffer: D3D12_BUFFER_SRV {
                    FirstElement: elements.start,
                    NumElements: elements.count() as u32,
                    StructureByteStride: structure_byte_stride,
                    Flags: D3D12_BUFFER_SRV_FLAGS(flags as i32),
                },
            },
            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
        })
    }

    #[inline]
    pub fn texture_1d(
        format: Format,
        most_detailed_mip: u32,
        mip_levels: u32,
        resource_min_lod_clamp: f32,
    ) -> Self {
        Self(D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format: format.into(),
            ViewDimension: D3D12_SRV_DIMENSION_TEXTURE1D,
            Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture1D: D3D12_TEX1D_SRV {
                    MostDetailedMip: most_detailed_mip,
                    MipLevels: mip_levels,
                    ResourceMinLODClamp: resource_min_lod_clamp,
                },
            },
            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
        })
    }

    #[inline]
    pub fn texture_2d(
        format: Format,
        most_detailed_mip: u32,
        mip_levels: u32,
        resource_min_lod_clamp: f32,
        plane_slice: u32,
    ) -> Self {
        Self(D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format: format.into(),
            ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2D,
            Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2D: D3D12_TEX2D_SRV {
                    MostDetailedMip: most_detailed_mip,
                    MipLevels: mip_levels,
                    ResourceMinLODClamp: resource_min_lod_clamp,
                    PlaneSlice: plane_slice,
                },
            },
            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
        })
    }

    #[inline]
    pub fn texture_3d(
        format: Format,
        most_detailed_mip: u32,
        mip_levels: u32,
        resource_min_lod_clamp: f32,
    ) -> Self {
        Self(D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format: format.into(),
            ViewDimension: D3D12_SRV_DIMENSION_TEXTURE3D,
            Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture3D: D3D12_TEX3D_SRV {
                    MostDetailedMip: most_detailed_mip,
                    MipLevels: mip_levels,
                    ResourceMinLODClamp: resource_min_lod_clamp,
                },
            },
            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
        })
    }

    #[inline]
    pub fn texture_1d_array(
        format: Format,
        most_detailed_mip: u32,
        mip_levels: u32,
        resource_min_lod_clamp: f32,
        array: Range<u32>,
    ) -> Self {
        Self(D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format: format.into(),
            ViewDimension: D3D12_SRV_DIMENSION_TEXTURE1DARRAY,
            Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture1DArray: D3D12_TEX1D_ARRAY_SRV {
                    MostDetailedMip: most_detailed_mip,
                    MipLevels: mip_levels,
                    ResourceMinLODClamp: resource_min_lod_clamp,
                    FirstArraySlice: array.start,
                    ArraySize: array.count() as u32,
                },
            },
            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
        })
    }

    #[inline]
    pub fn texture_2d_array(
        format: Format,
        most_detailed_mip: u32,
        mip_levels: u32,
        resource_min_lod_clamp: f32,
        plane_slice: u32,
        array: Range<u32>,
    ) -> Self {
        Self(D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format: format.into(),
            ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2DARRAY,
            Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2DArray: D3D12_TEX2D_ARRAY_SRV {
                    MostDetailedMip: most_detailed_mip,
                    MipLevels: mip_levels,
                    ResourceMinLODClamp: resource_min_lod_clamp,
                    PlaneSlice: plane_slice,
                    FirstArraySlice: array.start,
                    ArraySize: array.count() as u32,
                },
            },
            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
        })
    }

    #[inline]
    pub fn texture_2d_ms(format: Format) -> Self {
        Self(D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format: format.into(),
            ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2DMS,
            Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2DMS: D3D12_TEX2DMS_SRV::default(),
            },
            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
        })
    }

    #[inline]
    pub fn texture_2d_ms_array(format: Format, array: Range<u32>) -> Self {
        Self(D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format: format.into(),
            ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2DMSARRAY,
            Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2DMSArray: D3D12_TEX2DMS_ARRAY_SRV {
                    FirstArraySlice: array.start,
                    ArraySize: array.count() as u32,
                },
            },
            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
        })
    }

    #[inline]
    pub fn texture_cube(
        format: Format,
        most_detailed_mip: u32,
        mip_levels: u32,
        resource_min_lod_clamp: f32,
    ) -> Self {
        Self(D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format: format.into(),
            ViewDimension: D3D12_SRV_DIMENSION_TEXTURECUBE,
            Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                TextureCube: D3D12_TEXCUBE_SRV {
                    MostDetailedMip: most_detailed_mip,
                    MipLevels: mip_levels,
                    ResourceMinLODClamp: resource_min_lod_clamp,
                },
            },
            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
        })
    }

    #[inline]
    pub fn texture_cube_array(
        format: Format,
        most_detailed_mip: u32,
        mip_levels: u32,
        resource_min_lod_clamp: f32,
        array: Range<u32>,
    ) -> Self {
        Self(D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format: format.into(),
            ViewDimension: D3D12_SRV_DIMENSION_TEXTURECUBEARRAY,
            Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                TextureCubeArray: D3D12_TEXCUBE_ARRAY_SRV {
                    MostDetailedMip: most_detailed_mip,
                    MipLevels: mip_levels,
                    ResourceMinLODClamp: resource_min_lod_clamp,
                    First2DArrayFace: array.start,
                    NumCubes: array.count() as u32,
                },
            },
            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
        })
    }

    #[inline]
    pub fn raytracing_acceleration_structure(format: Format, location: GpuVirtualAddress) -> Self {
        Self(D3D12_SHADER_RESOURCE_VIEW_DESC {
            Format: format.into(),
            ViewDimension: D3D12_SRV_DIMENSION_RAYTRACING_ACCELERATION_STRUCTURE,
            Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
                RaytracingAccelerationStructure: D3D12_RAYTRACING_ACCELERATION_STRUCTURE_SRV {
                    Location: location.0,
                },
            },
            Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
        })
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct ComponentMapping(u32);

impl Default for ComponentMapping {
    fn default() -> Self {
        Self(D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING)
    }
}
