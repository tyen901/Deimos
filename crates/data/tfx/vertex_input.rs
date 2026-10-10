#[cfg(feature = "cpu")]
use crate::dxgi as d3d12;

pub struct InputElementFormat {
    pub hlsl_type: &'static str,
    pub stride: u32,
    pub format: d3d12::Format,
}

//region Built-in input layouts
pub const INPUT_SEMANTICS: [&str; 9] = [
    "POSITION",
    "BLENDWEIGHT",
    "BLENDINDICES",
    "NORMAL",
    "PSIZE",
    "TEXCOORD",
    "TANGENT",
    "BINORMAL",
    "COLOR",
];

#[rustfmt::skip]
pub const INPUT_FORMATS: [InputElementFormat; 34] = [
    InputElementFormat { hlsl_type: "", stride: 0, format: d3d12::Format::Unknown, }, // 0
    InputElementFormat { hlsl_type: "float", stride: 4, format: d3d12::Format::R32Float }, // 1
    InputElementFormat { hlsl_type: "float2", stride: 8, format: d3d12::Format::R32g32Float }, // 2
    InputElementFormat { hlsl_type: "float3", stride: 12, format: d3d12::Format::R32g32b32Float }, // 3
    InputElementFormat { hlsl_type: "float4", stride: 16, format: d3d12::Format::R32g32b32a32Float }, // 4
    InputElementFormat { hlsl_type: "float4", stride: 4, format: d3d12::Format::R8g8b8a8Unorm }, // 5
    InputElementFormat { hlsl_type: "uint4", stride: 4, format: d3d12::Format::R8g8b8a8Uint }, // 6
    InputElementFormat { hlsl_type: "int2", stride: 4, format: d3d12::Format::R16g16Sint }, // 7
    InputElementFormat { hlsl_type: "int4", stride: 8, format: d3d12::Format::R16g16b16a16Sint }, // 8
    InputElementFormat { hlsl_type: "uint4", stride: 8, format: d3d12::Format::R16g16b16a16Uint }, // 9
    InputElementFormat { hlsl_type: "float2", stride: 4, format: d3d12::Format::R16g16Snorm }, // 10
    InputElementFormat { hlsl_type: "float4", stride: 8, format: d3d12::Format::R16g16b16a16Snorm }, // 11
    InputElementFormat { hlsl_type: "float2", stride: 4, format: d3d12::Format::R16g16Float }, // 12
    InputElementFormat { hlsl_type: "float4", stride: 8, format: d3d12::Format::R16g16b16a16Float }, // 13
    InputElementFormat { hlsl_type: "int4", stride: 4, format: d3d12::Format::R8g8b8a8Sint }, // 14
    InputElementFormat { hlsl_type: "float4", stride: 4, format: d3d12::Format::R8g8b8a8Snorm }, // 15
    InputElementFormat { hlsl_type: "uint4", stride: 4, format: d3d12::Format::R10g10b10a2Uint }, // 16
    InputElementFormat { hlsl_type: "float4", stride: 4, format: d3d12::Format::R10g10b10a2Unorm }, // 17
    InputElementFormat { hlsl_type: "int", stride: 4, format: d3d12::Format::R32Sint }, // 18
    InputElementFormat { hlsl_type: "int2", stride: 8, format: d3d12::Format::R32g32Sint }, // 19
    InputElementFormat { hlsl_type: "int4", stride: 16, format: d3d12::Format::R32g32b32a32Sint }, // 20
    InputElementFormat { hlsl_type: "int", stride: 4, format: d3d12::Format::R32Uint }, // 21
    InputElementFormat { hlsl_type: "int2", stride: 8, format: d3d12::Format::R32g32Uint }, // 22
    InputElementFormat { hlsl_type: "int4", stride: 16, format: d3d12::Format::R32g32b32a32Uint }, // 23
    InputElementFormat { hlsl_type: "int", stride: 2, format: d3d12::Format::R16Sint }, // 24
    InputElementFormat { hlsl_type: "float", stride: 1, format: d3d12::Format::R8Unorm }, // 25
    InputElementFormat { hlsl_type: "", stride: 0, format: d3d12::Format::Unknown, }, // 26
    InputElementFormat { hlsl_type: "", stride: 0, format: d3d12::Format::Unknown, }, // 27
    InputElementFormat { hlsl_type: "", stride: 0, format: d3d12::Format::Unknown, }, // 28
    InputElementFormat { hlsl_type: "", stride: 0, format: d3d12::Format::Unknown, }, // 29
    InputElementFormat { hlsl_type: "", stride: 0, format: d3d12::Format::Unknown, }, // 30
    InputElementFormat { hlsl_type: "float4", stride: 4, format: d3d12::Format::R8g8b8a8UnormSrgb }, // 31
    InputElementFormat { hlsl_type: "float3", stride: 4, format: d3d12::Format::R11g11b10Float }, // 32
    InputElementFormat { hlsl_type: "float4", stride: 8, format: d3d12::Format::R16g16b16a16Snorm }, // 33
];

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VertexSemantic {
    Position,
    BlendWeight,
    BlendIndices,
    Normal,
    PointSize,
    TexCoord,
    Tangent,
    Binormal,
    Color,
}
/// Source input-layout identity, kept distinct from format and stream indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputLayoutIndex(pub u8);
impl InputLayoutIndex {
    pub const TERRAIN: Self = Self(22);
}
