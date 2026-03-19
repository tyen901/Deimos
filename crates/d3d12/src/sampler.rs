use bon::Builder;
use windows::Win32::Graphics::Direct3D12::*;

use crate::verify_ffi_type;

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum ComparisonFunc {
    Always = D3D12_COMPARISON_FUNC_ALWAYS.0,
    Equal = D3D12_COMPARISON_FUNC_EQUAL.0,
    Greater = D3D12_COMPARISON_FUNC_GREATER.0,
    GreaterEqual = D3D12_COMPARISON_FUNC_GREATER_EQUAL.0,
    Less = D3D12_COMPARISON_FUNC_LESS.0,
    LessEqual = D3D12_COMPARISON_FUNC_LESS_EQUAL.0,
    Never = D3D12_COMPARISON_FUNC_NEVER.0,
    None = D3D12_COMPARISON_FUNC_NONE.0,
    NotEqual = D3D12_COMPARISON_FUNC_NOT_EQUAL.0,
}
verify_ffi_type!(ComparisonFunc, D3D12_COMPARISON_FUNC);

#[repr(C)]
#[derive(Debug, Clone, Builder)]
pub struct SamplerDesc {
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
    #[builder(default = [0.0, 0.0, 0.0, 0.0])]
    pub border_color: [f32; 4],
    #[builder(default = 0.0)]
    pub min_lod: f32,
    #[builder(default = f32::MAX)]
    pub max_lod: f32,
}
verify_ffi_type!(SamplerDesc, D3D12_SAMPLER_DESC);

impl Default for SamplerDesc {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[repr(i32)]
#[rustfmt::skip]
#[derive(Debug, Clone, Copy)]
pub enum Filter {
    Anisotropic = D3D12_FILTER_ANISOTROPIC.0,
    ComparisonAnisotropic = D3D12_FILTER_COMPARISON_ANISOTROPIC.0,
    ComparisonMinLinearMagMipPoint = D3D12_FILTER_COMPARISON_MIN_LINEAR_MAG_MIP_POINT.0,
    ComparisonMinLinearMagPointMipLinear = D3D12_FILTER_COMPARISON_MIN_LINEAR_MAG_POINT_MIP_LINEAR.0,
    ComparisonMinMagAnisotropicMipPoint = D3D12_FILTER_COMPARISON_MIN_MAG_ANISOTROPIC_MIP_POINT.0,
    ComparisonMinMagLinearMipPoint = D3D12_FILTER_COMPARISON_MIN_MAG_LINEAR_MIP_POINT.0,
    ComparisonMinMagMipLinear = D3D12_FILTER_COMPARISON_MIN_MAG_MIP_LINEAR.0,
    ComparisonMinMagMipPoint = D3D12_FILTER_COMPARISON_MIN_MAG_MIP_POINT.0,
    ComparisonMinMagPointMipLinear = D3D12_FILTER_COMPARISON_MIN_MAG_POINT_MIP_LINEAR.0,
    ComparisonMinPointMagLinearMipPoint = D3D12_FILTER_COMPARISON_MIN_POINT_MAG_LINEAR_MIP_POINT.0,
    ComparisonMinPointMagMipLinear = D3D12_FILTER_COMPARISON_MIN_POINT_MAG_MIP_LINEAR.0,
    MaximumAnisotropic = D3D12_FILTER_MAXIMUM_ANISOTROPIC.0,
    MaximumMinLinearMagMipPoint = D3D12_FILTER_MAXIMUM_MIN_LINEAR_MAG_MIP_POINT.0,
    MaximumMinLinearMagPointMipLinear = D3D12_FILTER_MAXIMUM_MIN_LINEAR_MAG_POINT_MIP_LINEAR.0,
    MaximumMinMagAnisotropicMipPoint = D3D12_FILTER_MAXIMUM_MIN_MAG_ANISOTROPIC_MIP_POINT.0,
    MaximumMinMagLinearMipPoint = D3D12_FILTER_MAXIMUM_MIN_MAG_LINEAR_MIP_POINT.0,
    MaximumMinMagMipLinear = D3D12_FILTER_MAXIMUM_MIN_MAG_MIP_LINEAR.0,
    MaximumMinMagMipPoint = D3D12_FILTER_MAXIMUM_MIN_MAG_MIP_POINT.0,
    MaximumMinMagPointMipLinear = D3D12_FILTER_MAXIMUM_MIN_MAG_POINT_MIP_LINEAR.0,
    MaximumMinPointMagLinearMipPoint = D3D12_FILTER_MAXIMUM_MIN_POINT_MAG_LINEAR_MIP_POINT.0,
    MaximumMinPointMagMipLinear = D3D12_FILTER_MAXIMUM_MIN_POINT_MAG_MIP_LINEAR.0,
    MinimumAnisotropic = D3D12_FILTER_MINIMUM_ANISOTROPIC.0,
    MinimumMinLinearMagMipPoint = D3D12_FILTER_MINIMUM_MIN_LINEAR_MAG_MIP_POINT.0,
    MinimumMinLinearMagPointMipLinear = D3D12_FILTER_MINIMUM_MIN_LINEAR_MAG_POINT_MIP_LINEAR.0,
    MinimumMinMagAnisotropicMipPoint = D3D12_FILTER_MINIMUM_MIN_MAG_ANISOTROPIC_MIP_POINT.0,
    MinimumMinMagLinearMipPoint = D3D12_FILTER_MINIMUM_MIN_MAG_LINEAR_MIP_POINT.0,
    MinimumMinMagMipLinear = D3D12_FILTER_MINIMUM_MIN_MAG_MIP_LINEAR.0,
    MinimumMinMagMipPoint = D3D12_FILTER_MINIMUM_MIN_MAG_MIP_POINT.0,
    MinimumMinMagPointMipLinear = D3D12_FILTER_MINIMUM_MIN_MAG_POINT_MIP_LINEAR.0,
    MinimumMinPointMagLinearMipPoint = D3D12_FILTER_MINIMUM_MIN_POINT_MAG_LINEAR_MIP_POINT.0,
    MinimumMinPointMagMipLinear = D3D12_FILTER_MINIMUM_MIN_POINT_MAG_MIP_LINEAR.0,
    MinLinearMagMipPoint = D3D12_FILTER_MIN_LINEAR_MAG_MIP_POINT.0,
    MinLinearMagPointMipLinear = D3D12_FILTER_MIN_LINEAR_MAG_POINT_MIP_LINEAR.0,
    MinMagAnisotropicMipPoint = D3D12_FILTER_MIN_MAG_ANISOTROPIC_MIP_POINT.0,
    MinMagLinearMipPoint = D3D12_FILTER_MIN_MAG_LINEAR_MIP_POINT.0,
    MinMagMipLinear = D3D12_FILTER_MIN_MAG_MIP_LINEAR.0,
    MinMagMipPoint = D3D12_FILTER_MIN_MAG_MIP_POINT.0,
    MinMagPointMipLinear = D3D12_FILTER_MIN_MAG_POINT_MIP_LINEAR.0,
    MinPointMagLinearMipPoint = D3D12_FILTER_MIN_POINT_MAG_LINEAR_MIP_POINT.0,
    MinPointMagMipLinear = D3D12_FILTER_MIN_POINT_MAG_MIP_LINEAR.0,
}

#[repr(i32)]
#[rustfmt::skip]
#[derive(Debug, Clone, Copy)]
pub enum TextureAddressMode {
    Invalid = 0,
    Border = D3D12_TEXTURE_ADDRESS_MODE_BORDER.0,
    Clamp = D3D12_TEXTURE_ADDRESS_MODE_CLAMP.0,
    Mirror = D3D12_TEXTURE_ADDRESS_MODE_MIRROR.0,
    MirrorOnce = D3D12_TEXTURE_ADDRESS_MODE_MIRROR_ONCE.0,
    Wrap = D3D12_TEXTURE_ADDRESS_MODE_WRAP.0,
}
