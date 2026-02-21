use windows::Win32::Graphics::Dxgi::Common::*;

#[repr(i32)]
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Opaque420 = DXGI_FORMAT_420_OPAQUE.0,
    A4b4g4r4Unorm = DXGI_FORMAT_A4B4G4R4_UNORM.0,
    A8p8 = DXGI_FORMAT_A8P8.0,
    A8Unorm = DXGI_FORMAT_A8_UNORM.0,
    Ai44 = DXGI_FORMAT_AI44.0,
    Ayuv = DXGI_FORMAT_AYUV.0,
    B4g4r4a4Unorm = DXGI_FORMAT_B4G4R4A4_UNORM.0,
    B5g5r5a1Unorm = DXGI_FORMAT_B5G5R5A1_UNORM.0,
    B5g6r5Unorm = DXGI_FORMAT_B5G6R5_UNORM.0,
    B8g8r8a8Typeless = DXGI_FORMAT_B8G8R8A8_TYPELESS.0,
    B8g8r8a8Unorm = DXGI_FORMAT_B8G8R8A8_UNORM.0,
    B8g8r8a8UnormSrgb = DXGI_FORMAT_B8G8R8A8_UNORM_SRGB.0,
    B8g8r8x8Typeless = DXGI_FORMAT_B8G8R8X8_TYPELESS.0,
    B8g8r8x8Unorm = DXGI_FORMAT_B8G8R8X8_UNORM.0,
    B8g8r8x8UnormSrgb = DXGI_FORMAT_B8G8R8X8_UNORM_SRGB.0,
    Bc1Typeless = DXGI_FORMAT_BC1_TYPELESS.0,
    Bc1Unorm = DXGI_FORMAT_BC1_UNORM.0,
    Bc1UnormSrgb = DXGI_FORMAT_BC1_UNORM_SRGB.0,
    Bc2Typeless = DXGI_FORMAT_BC2_TYPELESS.0,
    Bc2Unorm = DXGI_FORMAT_BC2_UNORM.0,
    Bc2UnormSrgb = DXGI_FORMAT_BC2_UNORM_SRGB.0,
    Bc3Typeless = DXGI_FORMAT_BC3_TYPELESS.0,
    Bc3Unorm = DXGI_FORMAT_BC3_UNORM.0,
    Bc3UnormSrgb = DXGI_FORMAT_BC3_UNORM_SRGB.0,
    Bc4Snorm = DXGI_FORMAT_BC4_SNORM.0,
    Bc4Typeless = DXGI_FORMAT_BC4_TYPELESS.0,
    Bc4Unorm = DXGI_FORMAT_BC4_UNORM.0,
    Bc5Snorm = DXGI_FORMAT_BC5_SNORM.0,
    Bc5Typeless = DXGI_FORMAT_BC5_TYPELESS.0,
    Bc5Unorm = DXGI_FORMAT_BC5_UNORM.0,
    Bc6hSf16 = DXGI_FORMAT_BC6H_SF16.0,
    Bc6hTypeless = DXGI_FORMAT_BC6H_TYPELESS.0,
    Bc6hUf16 = DXGI_FORMAT_BC6H_UF16.0,
    Bc7Typeless = DXGI_FORMAT_BC7_TYPELESS.0,
    Bc7Unorm = DXGI_FORMAT_BC7_UNORM.0,
    Bc7UnormSrgb = DXGI_FORMAT_BC7_UNORM_SRGB.0,
    D16Unorm = DXGI_FORMAT_D16_UNORM.0,
    D24UnormS8Uint = DXGI_FORMAT_D24_UNORM_S8_UINT.0,
    D32Float = DXGI_FORMAT_D32_FLOAT.0,
    D32FloatS8x24Uint = DXGI_FORMAT_D32_FLOAT_S8X24_UINT.0,
    G8r8G8b8Unorm = DXGI_FORMAT_G8R8_G8B8_UNORM.0,
    Ia44 = DXGI_FORMAT_IA44.0,
    Nv11 = DXGI_FORMAT_NV11.0,
    Nv12 = DXGI_FORMAT_NV12.0,
    P010 = DXGI_FORMAT_P010.0,
    P016 = DXGI_FORMAT_P016.0,
    P208 = DXGI_FORMAT_P208.0,
    P8 = DXGI_FORMAT_P8.0,
    R10g10b10a2Typeless = DXGI_FORMAT_R10G10B10A2_TYPELESS.0,
    R10g10b10a2Uint = DXGI_FORMAT_R10G10B10A2_UINT.0,
    R10g10b10a2Unorm = DXGI_FORMAT_R10G10B10A2_UNORM.0,
    R10g10b10XrBiasA2Unorm = DXGI_FORMAT_R10G10B10_XR_BIAS_A2_UNORM.0,
    R11g11b10Float = DXGI_FORMAT_R11G11B10_FLOAT.0,
    R16g16b16a16Float = DXGI_FORMAT_R16G16B16A16_FLOAT.0,
    R16g16b16a16Sint = DXGI_FORMAT_R16G16B16A16_SINT.0,
    R16g16b16a16Snorm = DXGI_FORMAT_R16G16B16A16_SNORM.0,
    R16g16b16a16Typeless = DXGI_FORMAT_R16G16B16A16_TYPELESS.0,
    R16g16b16a16Uint = DXGI_FORMAT_R16G16B16A16_UINT.0,
    R16g16b16a16Unorm = DXGI_FORMAT_R16G16B16A16_UNORM.0,
    R16g16Float = DXGI_FORMAT_R16G16_FLOAT.0,
    R16g16Sint = DXGI_FORMAT_R16G16_SINT.0,
    R16g16Snorm = DXGI_FORMAT_R16G16_SNORM.0,
    R16g16Typeless = DXGI_FORMAT_R16G16_TYPELESS.0,
    R16g16Uint = DXGI_FORMAT_R16G16_UINT.0,
    R16g16Unorm = DXGI_FORMAT_R16G16_UNORM.0,
    R16Float = DXGI_FORMAT_R16_FLOAT.0,
    R16Sint = DXGI_FORMAT_R16_SINT.0,
    R16Snorm = DXGI_FORMAT_R16_SNORM.0,
    R16Typeless = DXGI_FORMAT_R16_TYPELESS.0,
    R16Uint = DXGI_FORMAT_R16_UINT.0,
    R16Unorm = DXGI_FORMAT_R16_UNORM.0,
    R1Unorm = DXGI_FORMAT_R1_UNORM.0,
    R24g8Typeless = DXGI_FORMAT_R24G8_TYPELESS.0,
    R24UnormX8Typeless = DXGI_FORMAT_R24_UNORM_X8_TYPELESS.0,
    R32g32b32a32Float = DXGI_FORMAT_R32G32B32A32_FLOAT.0,
    R32g32b32a32Sint = DXGI_FORMAT_R32G32B32A32_SINT.0,
    R32g32b32a32Typeless = DXGI_FORMAT_R32G32B32A32_TYPELESS.0,
    R32g32b32a32Uint = DXGI_FORMAT_R32G32B32A32_UINT.0,
    R32g32b32Float = DXGI_FORMAT_R32G32B32_FLOAT.0,
    R32g32b32Sint = DXGI_FORMAT_R32G32B32_SINT.0,
    R32g32b32Typeless = DXGI_FORMAT_R32G32B32_TYPELESS.0,
    R32g32b32Uint = DXGI_FORMAT_R32G32B32_UINT.0,
    R32g32Float = DXGI_FORMAT_R32G32_FLOAT.0,
    R32g32Sint = DXGI_FORMAT_R32G32_SINT.0,
    R32g32Typeless = DXGI_FORMAT_R32G32_TYPELESS.0,
    R32g32Uint = DXGI_FORMAT_R32G32_UINT.0,
    R32g8x24Typeless = DXGI_FORMAT_R32G8X24_TYPELESS.0,
    R32Float = DXGI_FORMAT_R32_FLOAT.0,
    R32FloatX8x24Typeless = DXGI_FORMAT_R32_FLOAT_X8X24_TYPELESS.0,
    R32Sint = DXGI_FORMAT_R32_SINT.0,
    R32Typeless = DXGI_FORMAT_R32_TYPELESS.0,
    R32Uint = DXGI_FORMAT_R32_UINT.0,
    R8g8b8a8Sint = DXGI_FORMAT_R8G8B8A8_SINT.0,
    R8g8b8a8Snorm = DXGI_FORMAT_R8G8B8A8_SNORM.0,
    R8g8b8a8Typeless = DXGI_FORMAT_R8G8B8A8_TYPELESS.0,
    R8g8b8a8Uint = DXGI_FORMAT_R8G8B8A8_UINT.0,
    R8g8b8a8Unorm = DXGI_FORMAT_R8G8B8A8_UNORM.0,
    R8g8b8a8UnormSrgb = DXGI_FORMAT_R8G8B8A8_UNORM_SRGB.0,
    R8g8B8g8Unorm = DXGI_FORMAT_R8G8_B8G8_UNORM.0,
    R8g8Sint = DXGI_FORMAT_R8G8_SINT.0,
    R8g8Snorm = DXGI_FORMAT_R8G8_SNORM.0,
    R8g8Typeless = DXGI_FORMAT_R8G8_TYPELESS.0,
    R8g8Uint = DXGI_FORMAT_R8G8_UINT.0,
    R8g8Unorm = DXGI_FORMAT_R8G8_UNORM.0,
    R8Sint = DXGI_FORMAT_R8_SINT.0,
    R8Snorm = DXGI_FORMAT_R8_SNORM.0,
    R8Typeless = DXGI_FORMAT_R8_TYPELESS.0,
    R8Uint = DXGI_FORMAT_R8_UINT.0,
    R8Unorm = DXGI_FORMAT_R8_UNORM.0,
    R9g9b9e5Sharedexp = DXGI_FORMAT_R9G9B9E5_SHAREDEXP.0,
    SamplerFeedbackMinMipOpaque = DXGI_FORMAT_SAMPLER_FEEDBACK_MIN_MIP_OPAQUE.0,
    SamplerFeedbackMipRegionUsedOpaque = DXGI_FORMAT_SAMPLER_FEEDBACK_MIP_REGION_USED_OPAQUE.0,
    #[default]
    Unknown = DXGI_FORMAT_UNKNOWN.0,
    V208 = DXGI_FORMAT_V208.0,
    V408 = DXGI_FORMAT_V408.0,
    X24TypelessG8Uint = DXGI_FORMAT_X24_TYPELESS_G8_UINT.0,
    X32TypelessG8x24Uint = DXGI_FORMAT_X32_TYPELESS_G8X24_UINT.0,
    Y210 = DXGI_FORMAT_Y210.0,
    Y216 = DXGI_FORMAT_Y216.0,
    Y410 = DXGI_FORMAT_Y410.0,
    Y416 = DXGI_FORMAT_Y416.0,
    Yuy2 = DXGI_FORMAT_YUY2.0,
}

impl Format {
    pub fn is_compressed(&self) -> bool {
        matches!(
            self,
            Format::Bc1Typeless
                | Format::Bc1Unorm
                | Format::Bc1UnormSrgb
                | Format::Bc2Typeless
                | Format::Bc2Unorm
                | Format::Bc2UnormSrgb
                | Format::Bc3Typeless
                | Format::Bc3Unorm
                | Format::Bc3UnormSrgb
                | Format::Bc4Typeless
                | Format::Bc4Unorm
                | Format::Bc4Snorm
                | Format::Bc5Typeless
                | Format::Bc5Unorm
                | Format::Bc5Snorm
                | Format::Bc6hTypeless
                | Format::Bc6hUf16
                | Format::Bc6hSf16
                | Format::Bc7Typeless
                | Format::Bc7Unorm
                | Format::Bc7UnormSrgb
        )
    }

    pub fn is_srgb(&self) -> bool {
        matches!(
            self,
            Format::B8g8r8a8UnormSrgb
                | Format::B8g8r8x8UnormSrgb
                | Format::Bc1UnormSrgb
                | Format::Bc2UnormSrgb
                | Format::Bc3UnormSrgb
                | Format::Bc7UnormSrgb
                | Format::R8g8b8a8UnormSrgb
        )
    }

    pub fn is_typeless(&self) -> bool {
        self.format_type() == Some(FormatType::Typeless)
    }

    pub fn is_unorm(&self) -> bool {
        self.format_type() == Some(FormatType::Unorm)
    }

    pub fn is_uint(&self) -> bool {
        self.format_type() == Some(FormatType::Uint)
    }

    pub fn is_float(&self) -> bool {
        self.format_type() == Some(FormatType::Float)
    }

    pub fn is_sint(&self) -> bool {
        self.format_type() == Some(FormatType::Sint)
    }

    pub fn is_snorm(&self) -> bool {
        self.format_type() == Some(FormatType::Snorm)
    }

    pub fn is_depth(&self) -> bool {
        matches!(
            self,
            Format::D16Unorm
                | Format::D24UnormS8Uint
                | Format::D32Float
                | Format::D32FloatS8x24Uint
                | Format::R32g8x24Typeless
        )
    }

    pub fn format_type(&self) -> Option<FormatType> {
        match self {
            Format::Bc4Snorm
            | Format::Bc5Snorm
            | Format::R16g16b16a16Snorm
            | Format::R16g16Snorm
            | Format::R16Snorm
            | Format::R8g8b8a8Snorm
            | Format::R8g8Snorm
            | Format::R8Snorm => Some(FormatType::Snorm),

            Format::A4b4g4r4Unorm
            | Format::A8Unorm
            | Format::B4g4r4a4Unorm
            | Format::B5g5r5a1Unorm
            | Format::B5g6r5Unorm
            | Format::B8g8r8a8Unorm
            | Format::B8g8r8a8UnormSrgb
            | Format::B8g8r8x8Unorm
            | Format::B8g8r8x8UnormSrgb
            | Format::Bc1Unorm
            | Format::Bc1UnormSrgb
            | Format::Bc2Unorm
            | Format::Bc2UnormSrgb
            | Format::Bc3Unorm
            | Format::Bc3UnormSrgb
            | Format::Bc4Unorm
            | Format::Bc5Unorm
            | Format::Bc7Unorm
            | Format::Bc7UnormSrgb
            | Format::D16Unorm
            | Format::D24UnormS8Uint
            | Format::G8r8G8b8Unorm
            | Format::R10g10b10a2Unorm
            | Format::R10g10b10XrBiasA2Unorm
            | Format::R16g16b16a16Unorm
            | Format::R16g16Unorm
            | Format::R16Unorm
            | Format::R1Unorm
            | Format::R24UnormX8Typeless
            | Format::R8g8b8a8Unorm
            | Format::R8g8b8a8UnormSrgb
            | Format::R8g8B8g8Unorm
            | Format::R8g8Unorm
            | Format::R8Unorm => Some(FormatType::Unorm),

            Format::R16g16b16a16Sint
            | Format::R16g16Sint
            | Format::R16Sint
            | Format::R32g32b32a32Sint
            | Format::R32g32b32Sint
            | Format::R32g32Sint
            | Format::R32Sint
            | Format::R8g8b8a8Sint
            | Format::R8g8Sint
            | Format::R8Sint => Some(FormatType::Sint),

            Format::D32FloatS8x24Uint
            | Format::R10g10b10a2Uint
            | Format::R16g16b16a16Uint
            | Format::R16g16Uint
            | Format::R16Uint
            | Format::R32g32b32a32Uint
            | Format::R32g32b32Uint
            | Format::R32g32Uint
            | Format::R32Uint
            | Format::R8g8b8a8Uint
            | Format::R8g8Uint
            | Format::R8Uint => Some(FormatType::Uint),

            Format::D32Float
            | Format::R11g11b10Float
            | Format::R16g16b16a16Float
            | Format::R16g16Float
            | Format::R16Float
            | Format::R32g32b32a32Float
            | Format::R32g32b32Float
            | Format::R32g32Float
            | Format::R32Float
            | Format::R32FloatX8x24Typeless => Some(FormatType::Float),

            Format::B8g8r8a8Typeless
            | Format::B8g8r8x8Typeless
            | Format::Bc1Typeless
            | Format::Bc2Typeless
            | Format::Bc3Typeless
            | Format::Bc4Typeless
            | Format::Bc5Typeless
            | Format::Bc6hTypeless
            | Format::Bc7Typeless
            | Format::R10g10b10a2Typeless
            | Format::R16g16b16a16Typeless
            | Format::R16g16Typeless
            | Format::R16Typeless
            | Format::R24g8Typeless
            | Format::R32g32b32a32Typeless
            | Format::R32g32b32Typeless
            | Format::R32g32Typeless
            | Format::R32g8x24Typeless
            | Format::R32Typeless
            | Format::R8g8b8a8Typeless
            | Format::R8g8Typeless
            | Format::R8Typeless
            | Format::X24TypelessG8Uint
            | Format::X32TypelessG8x24Uint => Some(FormatType::Typeless),

            Format::Opaque420
            | Format::A8p8
            | Format::Ai44
            | Format::Ayuv
            | Format::Bc6hSf16
            | Format::Bc6hUf16
            | Format::Ia44
            | Format::Nv11
            | Format::Nv12
            | Format::P010
            | Format::P016
            | Format::P208
            | Format::P8
            | Format::R9g9b9e5Sharedexp
            | Format::SamplerFeedbackMinMipOpaque
            | Format::SamplerFeedbackMipRegionUsedOpaque
            | Format::Unknown
            | Format::V208
            | Format::V408
            | Format::Y210
            | Format::Y216
            | Format::Y410
            | Format::Y416
            | Format::Yuy2 => None,
        }
    }
}

impl From<Format> for DXGI_FORMAT {
    fn from(format: Format) -> Self {
        Self(format as i32)
    }
}

// Use a checked transmute for this
impl TryFrom<u32> for Format {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value >= DXGI_FORMAT_UNKNOWN.0 as u32 && value <= DXGI_FORMAT_YUY2.0 as u32 {
            Ok(unsafe { std::mem::transmute::<u32, Format>(value) })
        } else {
            Err(())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatType {
    Unorm,
    Snorm,
    Uint,
    Sint,
    Float,
    Typeless,
}
