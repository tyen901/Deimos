use std::{marker::PhantomData, mem::transmute};

use bitflags::bitflags;
use bon::Builder;
use windows::Win32::Graphics::Direct3D12::*;

use crate::{
    root_signature::static_sampler_desc_builder::{SetRegisterSpace, SetShaderRegister},
    util::blob_to_vec,
    verify_ffi_type, ComparisonFunc, Filter, Result, TextureAddressMode,
};

#[repr(transparent)]
#[derive(Clone)]
pub struct RootSignature(pub(crate) ID3D12RootSignature);

#[derive(Default)]
pub struct RootSignatureBuilder<'a> {
    parameters: Vec<D3D12_ROOT_PARAMETER>,
    static_samplers: Vec<D3D12_STATIC_SAMPLER_DESC>,
    flags: RootSignatureFlags,

    _marker: PhantomData<&'a ()>,
}

impl<'a> RootSignatureBuilder<'a> {
    pub fn with_param(mut self, param: RootParameter<'a>, visibility: ShaderVisibility) -> Self {
        self.add_param(param, visibility);
        self
    }

    pub fn add_param(&mut self, param: RootParameter<'a>, visibility: ShaderVisibility) -> u32 {
        let index = self.parameters.len();
        let param_ffi = match param {
            RootParameter::DescriptorTable(descriptor_ranges) => D3D12_ROOT_PARAMETER {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE,
                Anonymous: D3D12_ROOT_PARAMETER_0 {
                    DescriptorTable: D3D12_ROOT_DESCRIPTOR_TABLE {
                        NumDescriptorRanges: descriptor_ranges.len() as u32,
                        pDescriptorRanges: descriptor_ranges.as_ptr().cast(),
                    },
                },
                ShaderVisibility: visibility.into(),
            },
            RootParameter::Constants {
                shader_register,
                register_space,
                num_32bit_values,
            } => D3D12_ROOT_PARAMETER {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS,
                Anonymous: D3D12_ROOT_PARAMETER_0 {
                    Constants: D3D12_ROOT_CONSTANTS {
                        ShaderRegister: shader_register,
                        RegisterSpace: register_space,
                        Num32BitValues: num_32bit_values,
                    },
                },
                ShaderVisibility: visibility.into(),
            },
            RootParameter::CbvDescriptor {
                shader_register,
                register_space,
            } => D3D12_ROOT_PARAMETER {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE_CBV,
                Anonymous: D3D12_ROOT_PARAMETER_0 {
                    Descriptor: D3D12_ROOT_DESCRIPTOR {
                        ShaderRegister: shader_register,
                        RegisterSpace: register_space,
                    },
                },
                ShaderVisibility: visibility.into(),
            },
            RootParameter::SrvDescriptor {
                shader_register,
                register_space,
            } => D3D12_ROOT_PARAMETER {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE_SRV,
                Anonymous: D3D12_ROOT_PARAMETER_0 {
                    Descriptor: D3D12_ROOT_DESCRIPTOR {
                        ShaderRegister: shader_register,
                        RegisterSpace: register_space,
                    },
                },
                ShaderVisibility: visibility.into(),
            },
            RootParameter::UavDescriptor {
                shader_register,
                register_space,
            } => D3D12_ROOT_PARAMETER {
                ParameterType: D3D12_ROOT_PARAMETER_TYPE_UAV,
                Anonymous: D3D12_ROOT_PARAMETER_0 {
                    Descriptor: D3D12_ROOT_DESCRIPTOR {
                        ShaderRegister: shader_register,
                        RegisterSpace: register_space,
                    },
                },
                ShaderVisibility: visibility.into(),
            },
        };

        self.parameters.push(param_ffi);
        index as u32
    }

    pub fn with_sampler(mut self, desc: StaticSamplerDesc) -> Self {
        self.add_sampler(desc);
        self
    }

    pub fn add_sampler(&mut self, desc: StaticSamplerDesc) {
        self.static_samplers
            .push(unsafe { transmute::<StaticSamplerDesc, D3D12_STATIC_SAMPLER_DESC>(desc) });
    }

    pub const fn flags(mut self, flags: RootSignatureFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn serialize(self) -> Result<Vec<u8>> {
        let mut blob = None;
        let mut error_blob = None;

        let desc = D3D12_ROOT_SIGNATURE_DESC {
            NumParameters: self.parameters.len() as u32,
            pParameters: self.parameters.as_ptr(),
            NumStaticSamplers: self.static_samplers.len() as u32,
            pStaticSamplers: self.static_samplers.as_ptr(),
            Flags: D3D12_ROOT_SIGNATURE_FLAGS(self.flags.bits()),
        };

        let res = unsafe {
            D3D12SerializeRootSignature(
                &raw const desc,
                D3D_ROOT_SIGNATURE_VERSION_1_0,
                &raw mut blob,
                Some(&raw mut error_blob),
            )
        };

        match res {
            Ok(()) => {
                let blob = blob.expect("blob should be non-null on success");
                Ok(blob_to_vec(blob))
            }
            Err(_err) => {
                let error_blob = error_blob.expect("error blob should be non-null on failure");
                let data = blob_to_vec(error_blob);
                let error_blob_string = String::from_utf8_lossy(&data);
                Err(crate::Error::Other(error_blob_string.to_string()))
            }
        }
    }
}

impl TryInto<Vec<u8>> for RootSignatureBuilder<'_> {
    type Error = crate::Error;

    fn try_into(self) -> std::result::Result<Vec<u8>, Self::Error> {
        self.serialize()
    }
}

#[repr(C)]
#[derive(Debug, Builder, Clone)]
#[builder(start_fn = "_builder_internal")]
pub struct StaticSamplerDesc {
    #[builder(default = Filter::MinMagMipLinear)]
    pub filter: Filter,
    #[builder(default = TextureAddressMode::Clamp)]
    pub address_u: TextureAddressMode,
    #[builder(default = TextureAddressMode::Clamp)]
    pub address_v: TextureAddressMode,
    #[builder(default = TextureAddressMode::Clamp)]
    pub address_w: TextureAddressMode,
    #[builder(default = 0.0)]
    pub mip_lod_bias: f32,
    #[builder(default = 1)]
    pub max_anisotropy: u32,
    #[builder(default = ComparisonFunc::Always)]
    pub comparison_func: ComparisonFunc,
    #[builder(default = D3D12_STATIC_BORDER_COLOR_TRANSPARENT_BLACK)]
    pub border_color: D3D12_STATIC_BORDER_COLOR,
    #[builder(default = 0.0)]
    pub min_lod: f32,
    #[builder(default = f32::MAX)]
    pub max_lod: f32,

    pub shader_register: u32,
    pub register_space: u32,

    #[builder(default = ShaderVisibility::All)]
    pub shader_visibility: ShaderVisibility,
}
verify_ffi_type!(StaticSamplerDesc, D3D12_STATIC_SAMPLER_DESC);

impl StaticSamplerDesc {
    pub fn builder(
        shader_register: u32,
        register_space: u32,
    ) -> StaticSamplerDescBuilder<SetRegisterSpace<SetShaderRegister>> {
        Self::_builder_internal()
            .shader_register(shader_register)
            .register_space(register_space)
    }
}

pub enum RootParameter<'a> {
    DescriptorTable(&'a [DescriptorRange]),
    Constants {
        shader_register: u32,
        register_space: u32,
        num_32bit_values: u32,
    },
    CbvDescriptor {
        shader_register: u32,
        register_space: u32,
    },
    SrvDescriptor {
        shader_register: u32,
        register_space: u32,
    },
    UavDescriptor {
        shader_register: u32,
        register_space: u32,
    },
}

#[repr(C)]
pub struct DescriptorRange {
    pub range_type: DescriptorRangeType,
    pub num_descriptors: u32,
    pub base_shader_register: u32,
    pub register_space: u32,
    pub offset_in_descriptors_from_table_start: u32,
}
verify_ffi_type!(DescriptorRange, D3D12_DESCRIPTOR_RANGE);

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DescriptorRangeType {
    Srv = D3D12_DESCRIPTOR_RANGE_TYPE_SRV.0,
    Uav = D3D12_DESCRIPTOR_RANGE_TYPE_UAV.0,
    Cbv = D3D12_DESCRIPTOR_RANGE_TYPE_CBV.0,
    Sampler = D3D12_DESCRIPTOR_RANGE_TYPE_SAMPLER.0,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderVisibility {
    All = D3D12_SHADER_VISIBILITY_ALL.0,
    Amplification = D3D12_SHADER_VISIBILITY_AMPLIFICATION.0,
    Domain = D3D12_SHADER_VISIBILITY_DOMAIN.0,
    Geometry = D3D12_SHADER_VISIBILITY_GEOMETRY.0,
    Hull = D3D12_SHADER_VISIBILITY_HULL.0,
    Mesh = D3D12_SHADER_VISIBILITY_MESH.0,
    Pixel = D3D12_SHADER_VISIBILITY_PIXEL.0,
    Vertex = D3D12_SHADER_VISIBILITY_VERTEX.0,
}

impl From<ShaderVisibility> for D3D12_SHADER_VISIBILITY {
    fn from(value: ShaderVisibility) -> Self {
        Self(value as i32)
    }
}

bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct RootSignatureFlags : i32 {
        const ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT = D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT.0;
        const ALLOW_STREAM_OUTPUT = D3D12_ROOT_SIGNATURE_FLAG_ALLOW_STREAM_OUTPUT.0;
        const CBV_SRV_UAV_HEAP_DIRECTLY_INDEXED = D3D12_ROOT_SIGNATURE_FLAG_CBV_SRV_UAV_HEAP_DIRECTLY_INDEXED.0;
        const DENY_AMPLIFICATION_SHADER_ROOT_ACCESS = D3D12_ROOT_SIGNATURE_FLAG_DENY_AMPLIFICATION_SHADER_ROOT_ACCESS.0;
        const DENY_DOMAIN_SHADER_ROOT_ACCESS = D3D12_ROOT_SIGNATURE_FLAG_DENY_DOMAIN_SHADER_ROOT_ACCESS.0;
        const DENY_GEOMETRY_SHADER_ROOT_ACCESS = D3D12_ROOT_SIGNATURE_FLAG_DENY_GEOMETRY_SHADER_ROOT_ACCESS.0;
        const DENY_HULL_SHADER_ROOT_ACCESS = D3D12_ROOT_SIGNATURE_FLAG_DENY_HULL_SHADER_ROOT_ACCESS.0;
        const DENY_MESH_SHADER_ROOT_ACCESS = D3D12_ROOT_SIGNATURE_FLAG_DENY_MESH_SHADER_ROOT_ACCESS.0;
        const DENY_PIXEL_SHADER_ROOT_ACCESS = D3D12_ROOT_SIGNATURE_FLAG_DENY_PIXEL_SHADER_ROOT_ACCESS.0;
        const DENY_VERTEX_SHADER_ROOT_ACCESS = D3D12_ROOT_SIGNATURE_FLAG_DENY_VERTEX_SHADER_ROOT_ACCESS.0;
        const LOCAL_ROOT_SIGNATURE = D3D12_ROOT_SIGNATURE_FLAG_LOCAL_ROOT_SIGNATURE.0;
        const SAMPLER_HEAP_DIRECTLY_INDEXED = D3D12_ROOT_SIGNATURE_FLAG_SAMPLER_HEAP_DIRECTLY_INDEXED.0;
    }
}
