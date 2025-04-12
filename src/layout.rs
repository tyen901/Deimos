use d3d11::dxgi;
use gltf::{
    accessor::{ComponentType, Type},
    mesh::Semantic,
};

pub enum TigerInputSemantic {
    Position,
    BlendWeight,
    BlendIndices,
    Normal,
    PSize,
    TexCoord,
    Tangent,
    BiNormal,
    Color,
}

/// The boolean indicates whether the input element is normalized.
#[rustfmt::skip]
pub const INPUT_FORMATS: [Option<(ComponentType, Type, bool)>; 34] = [
    // InputElementFormat { hlsl_type: "", stride: 0, format: dxgi::Format::Unknown, }, // 0
    Some((ComponentType::U8, Type::Scalar, false)), // 0, null
    // InputElementFormat { hlsl_type: "float", stride: 4, format: dxgi::Format::R32Float }, // 1
    Some((ComponentType::F32, Type::Scalar, false)),
    // InputElementFormat { hlsl_type: "float2", stride: 8, format: dxgi::Format::R32g32Float }, // 2
    Some((ComponentType::F32, Type::Vec2, false)),
    // InputElementFormat { hlsl_type: "float3", stride: 12, format: dxgi::Format::R32g32b32Float }, // 3
    Some((ComponentType::F32, Type::Vec3, false)),
    // InputElementFormat { hlsl_type: "float4", stride: 16, format: dxgi::Format::R32g32b32a32Float }, // 4
    Some((ComponentType::F32, Type::Vec4, false)),
    // InputElementFormat { hlsl_type: "float4", stride: 4, format: dxgi::Format::R8g8b8a8Unorm }, // 5
    Some((ComponentType::U8, Type::Vec4, true))     ,
    // InputElementFormat { hlsl_type: "uint4", stride: 4, format: dxgi::Format::R8g8b8a8Uint }, // 6
    Some((ComponentType::U8, Type::Vec4, false)),
    // InputElementFormat { hlsl_type: "int2", stride: 4, format: dxgi::Format::R16g16Sint }, // 7
    Some((ComponentType::I16, Type::Vec2, false)),
    // InputElementFormat { hlsl_type: "int4", stride: 8, format: dxgi::Format::R16g16b16a16Sint }, // 8
    None,
    // InputElementFormat { hlsl_type: "uint4", stride: 8, format: dxgi::Format::R16g16b16a16Uint }, // 9
    None,
    // InputElementFormat { hlsl_type: "float2", stride: 4, format: dxgi::Format::R16g16Snorm }, // 10
    None,
    // InputElementFormat { hlsl_type: "float4", stride: 8, format: dxgi::Format::R16g16b16a16Snorm }, // 11
    None,
    // InputElementFormat { hlsl_type: "float2", stride: 4, format: dxgi::Format::R16g16Float }, // 12
    None,
    // InputElementFormat { hlsl_type: "float4", stride: 8, format: dxgi::Format::R16g16b16a16Float }, // 13
    None,
    // InputElementFormat { hlsl_type: "int4", stride: 4, format: dxgi::Format::R8g8b8a8Sint }, // 14
    None,
    // InputElementFormat { hlsl_type: "float4", stride: 4, format: dxgi::Format::R8g8b8a8Snorm }, // 15
    None,
    // InputElementFormat { hlsl_type: "uint4", stride: 4, format: dxgi::Format::R10g10b10a2Uint }, // 16
    None,
    // InputElementFormat { hlsl_type: "float4", stride: 4, format: dxgi::Format::R10g10b10a2Unorm }, // 17
    None,
    // InputElementFormat { hlsl_type: "int", stride: 4, format: dxgi::Format::R32Sint }, // 18
    None,
    // InputElementFormat { hlsl_type: "int2", stride: 8, format: dxgi::Format::R32g32Sint }, // 19
    None,
    // InputElementFormat { hlsl_type: "int4", stride: 16, format: dxgi::Format::R32g32b32a32Sint }, // 20
    None,
    // InputElementFormat { hlsl_type: "int", stride: 4, format: dxgi::Format::R32Uint }, // 21
    None,
    // InputElementFormat { hlsl_type: "int2", stride: 8, format: dxgi::Format::R32g32Uint }, // 22
    None,
    // InputElementFormat { hlsl_type: "int4", stride: 16, format: dxgi::Format::R32g32b32a32Uint }, // 23
    None,
    // InputElementFormat { hlsl_type: "int", stride: 2, format: dxgi::Format::R16Sint }, // 24
    None,
    // InputElementFormat { hlsl_type: "float", stride: 1, format: dxgi::Format::R8Unorm }, // 25
    None,
    // InputElementFormat { hlsl_type: "", stride: 0, format: dxgi::Format::Unknown, }, // 26
    None,
    // InputElementFormat { hlsl_type: "", stride: 0, format: dxgi::Format::Unknown, }, // 27
    None,
    // InputElementFormat { hlsl_type: "", stride: 0, format: dxgi::Format::Unknown, }, // 28
    None,
    // InputElementFormat { hlsl_type: "", stride: 0, format: dxgi::Format::Unknown, }, // 29
    None,
    // InputElementFormat { hlsl_type: "", stride: 0, format: dxgi::Format::Unknown, }, // 30
    None,
    // InputElementFormat { hlsl_type: "float4", stride: 4, format: dxgi::Format::R8g8b8a8UnormSrgb }, // 31
    None,
    // InputElementFormat { hlsl_type: "float3", stride: 4, format: dxgi::Format::R11g11b10Float }, // 32
    None,
    // InputElementFormat { hlsl_type: "float4", stride: 8, format: dxgi::Format::R16g16b16a16Snorm }, // 33
    None,
];

struct TigerInputLayout {
    pub elements: &'static [TigerInputLayoutElement],
}

struct TigerInputLayoutElement {
    pub hlsl_type: &'static str,
    pub format: dxgi::Format,
    pub stride: u32,
    pub semantic_name: &'static str,
    pub semantic_index: u32,
    pub buffer_index: u32,
    pub is_instance_data: bool,
}

const BASE_INPUT_LAYOUTS: [TigerInputLayout; 7] = [
    // Layout 0
    TigerInputLayout {
        elements: &[TigerInputLayoutElement {
            hlsl_type: "float3",
            format: dxgi::Format::R32g32b32Float,
            stride: 12,
            semantic_name: "POSITION",
            semantic_index: 0,
            buffer_index: 0,
            is_instance_data: false,
        }],
    },
    // Layout 1
    TigerInputLayout {
        elements: &[TigerInputLayoutElement {
            hlsl_type: "float3",
            format: dxgi::Format::R32g32b32Float,
            stride: 12,
            semantic_name: "POSITION",
            semantic_index: 0,
            buffer_index: 0,
            is_instance_data: false,
        }],
    },
    // Layout 2
    TigerInputLayout {
        elements: &[
            TigerInputLayoutElement {
                hlsl_type: "float2",
                format: dxgi::Format::R32g32Float,
                stride: 8,
                semantic_name: "POSITION",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float2",
                format: dxgi::Format::R32g32Float,
                stride: 8,
                semantic_name: "TEXCOORD",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float4",
                format: dxgi::Format::R8g8b8a8Unorm,
                stride: 4,
                semantic_name: "COLOR",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
        ],
    },
    // Layout 3
    TigerInputLayout {
        elements: &[
            TigerInputLayoutElement {
                hlsl_type: "float3",
                format: dxgi::Format::R32g32b32Float,
                stride: 12,
                semantic_name: "POSITION",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float2",
                format: dxgi::Format::R32g32Float,
                stride: 8,
                semantic_name: "TEXCOORD",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float4",
                format: dxgi::Format::R8g8b8a8Unorm,
                stride: 4,
                semantic_name: "COLOR",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
        ],
    },
    // Layout 4
    TigerInputLayout {
        elements: &[
            TigerInputLayoutElement {
                hlsl_type: "float3",
                format: dxgi::Format::R32g32b32Float,
                stride: 12,
                semantic_name: "POSITION",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float4",
                format: dxgi::Format::R8g8b8a8Unorm,
                stride: 4,
                semantic_name: "COLOR",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
        ],
    },
    // Layout 5
    TigerInputLayout {
        elements: &[
            TigerInputLayoutElement {
                hlsl_type: "float2",
                format: dxgi::Format::R32g32Float,
                stride: 8,
                semantic_name: "POSITION",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float2",
                format: dxgi::Format::R32g32Float,
                stride: 8,
                semantic_name: "TEXCOORD",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
        ],
    },
    // Layout 6
    TigerInputLayout {
        elements: &[
            TigerInputLayoutElement {
                hlsl_type: "float3",
                format: dxgi::Format::R32g32b32Float,
                stride: 12,
                semantic_name: "POSITION",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float3",
                format: dxgi::Format::R32g32b32Float,
                stride: 12,
                semantic_name: "NORMAL",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float4",
                format: dxgi::Format::R32g32b32a32Float,
                stride: 16,
                semantic_name: "TANGENT",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
            TigerInputLayoutElement {
                hlsl_type: "float2",
                format: dxgi::Format::R32g32Float,
                stride: 8,
                semantic_name: "TEXCOORD",
                semantic_index: 0,
                buffer_index: 0,
                is_instance_data: false,
            },
        ],
    },
];
