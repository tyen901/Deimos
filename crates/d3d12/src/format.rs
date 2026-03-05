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
    pub const fn is_compressed(&self) -> bool {
        matches!(
            self,
            Self::Bc1Typeless
                | Self::Bc1Unorm
                | Self::Bc1UnormSrgb
                | Self::Bc2Typeless
                | Self::Bc2Unorm
                | Self::Bc2UnormSrgb
                | Self::Bc3Typeless
                | Self::Bc3Unorm
                | Self::Bc3UnormSrgb
                | Self::Bc4Typeless
                | Self::Bc4Unorm
                | Self::Bc4Snorm
                | Self::Bc5Typeless
                | Self::Bc5Unorm
                | Self::Bc5Snorm
                | Self::Bc6hTypeless
                | Self::Bc6hUf16
                | Self::Bc6hSf16
                | Self::Bc7Typeless
                | Self::Bc7Unorm
                | Self::Bc7UnormSrgb
        )
    }

    pub const fn is_srgb(&self) -> bool {
        matches!(
            self,
            Self::B8g8r8a8UnormSrgb
                | Self::B8g8r8x8UnormSrgb
                | Self::Bc1UnormSrgb
                | Self::Bc2UnormSrgb
                | Self::Bc3UnormSrgb
                | Self::Bc7UnormSrgb
                | Self::R8g8b8a8UnormSrgb
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

    pub const fn is_depth(&self) -> bool {
        matches!(
            self,
            Self::D16Unorm
                | Self::D24UnormS8Uint
                | Self::D32Float
                | Self::D32FloatS8x24Uint
                | Self::R32g8x24Typeless
        )
    }

    pub const fn format_type(&self) -> Option<FormatType> {
        match self {
            Self::Bc4Snorm
            | Self::Bc5Snorm
            | Self::R16g16b16a16Snorm
            | Self::R16g16Snorm
            | Self::R16Snorm
            | Self::R8g8b8a8Snorm
            | Self::R8g8Snorm
            | Self::R8Snorm => Some(FormatType::Snorm),

            Self::A4b4g4r4Unorm
            | Self::A8Unorm
            | Self::B4g4r4a4Unorm
            | Self::B5g5r5a1Unorm
            | Self::B5g6r5Unorm
            | Self::B8g8r8a8Unorm
            | Self::B8g8r8a8UnormSrgb
            | Self::B8g8r8x8Unorm
            | Self::B8g8r8x8UnormSrgb
            | Self::Bc1Unorm
            | Self::Bc1UnormSrgb
            | Self::Bc2Unorm
            | Self::Bc2UnormSrgb
            | Self::Bc3Unorm
            | Self::Bc3UnormSrgb
            | Self::Bc4Unorm
            | Self::Bc5Unorm
            | Self::Bc7Unorm
            | Self::Bc7UnormSrgb
            | Self::D16Unorm
            | Self::D24UnormS8Uint
            | Self::G8r8G8b8Unorm
            | Self::R10g10b10a2Unorm
            | Self::R10g10b10XrBiasA2Unorm
            | Self::R16g16b16a16Unorm
            | Self::R16g16Unorm
            | Self::R16Unorm
            | Self::R1Unorm
            | Self::R24UnormX8Typeless
            | Self::R8g8b8a8Unorm
            | Self::R8g8b8a8UnormSrgb
            | Self::R8g8B8g8Unorm
            | Self::R8g8Unorm
            | Self::R8Unorm => Some(FormatType::Unorm),

            Self::R16g16b16a16Sint
            | Self::R16g16Sint
            | Self::R16Sint
            | Self::R32g32b32a32Sint
            | Self::R32g32b32Sint
            | Self::R32g32Sint
            | Self::R32Sint
            | Self::R8g8b8a8Sint
            | Self::R8g8Sint
            | Self::R8Sint => Some(FormatType::Sint),

            Self::D32FloatS8x24Uint
            | Self::R10g10b10a2Uint
            | Self::R16g16b16a16Uint
            | Self::R16g16Uint
            | Self::R16Uint
            | Self::R32g32b32a32Uint
            | Self::R32g32b32Uint
            | Self::R32g32Uint
            | Self::R32Uint
            | Self::R8g8b8a8Uint
            | Self::R8g8Uint
            | Self::R8Uint => Some(FormatType::Uint),

            Self::D32Float
            | Self::R11g11b10Float
            | Self::R16g16b16a16Float
            | Self::R16g16Float
            | Self::R16Float
            | Self::R32g32b32a32Float
            | Self::R32g32b32Float
            | Self::R32g32Float
            | Self::R32Float
            | Self::R32FloatX8x24Typeless => Some(FormatType::Float),

            Self::B8g8r8a8Typeless
            | Self::B8g8r8x8Typeless
            | Self::Bc1Typeless
            | Self::Bc2Typeless
            | Self::Bc3Typeless
            | Self::Bc4Typeless
            | Self::Bc5Typeless
            | Self::Bc6hTypeless
            | Self::Bc7Typeless
            | Self::R10g10b10a2Typeless
            | Self::R16g16b16a16Typeless
            | Self::R16g16Typeless
            | Self::R16Typeless
            | Self::R24g8Typeless
            | Self::R32g32b32a32Typeless
            | Self::R32g32b32Typeless
            | Self::R32g32Typeless
            | Self::R32g8x24Typeless
            | Self::R32Typeless
            | Self::R8g8b8a8Typeless
            | Self::R8g8Typeless
            | Self::R8Typeless
            | Self::X24TypelessG8Uint
            | Self::X32TypelessG8x24Uint => Some(FormatType::Typeless),

            Self::Opaque420
            | Self::A8p8
            | Self::Ai44
            | Self::Ayuv
            | Self::Bc6hSf16
            | Self::Bc6hUf16
            | Self::Ia44
            | Self::Nv11
            | Self::Nv12
            | Self::P010
            | Self::P016
            | Self::P208
            | Self::P8
            | Self::R9g9b9e5Sharedexp
            | Self::SamplerFeedbackMinMipOpaque
            | Self::SamplerFeedbackMipRegionUsedOpaque
            | Self::Unknown
            | Self::V208
            | Self::V408
            | Self::Y210
            | Self::Y216
            | Self::Y410
            | Self::Y416
            | Self::Yuy2 => None,
        }
    }

    #[allow(clippy::match_same_arms)]
    pub fn bpp(&self) -> u32 {
        match self {
            Self::R32g32b32a32Typeless
            | Self::R32g32b32a32Float
            | Self::R32g32b32a32Uint
            | Self::R32g32b32a32Sint => 128,
            Self::R32g32b32Typeless
            | Self::R32g32b32Float
            | Self::R32g32b32Uint
            | Self::R32g32b32Sint => 96,
            Self::R16g16b16a16Typeless
            | Self::R16g16b16a16Float
            | Self::R16g16b16a16Unorm
            | Self::R16g16b16a16Uint
            | Self::R16g16b16a16Snorm
            | Self::R16g16b16a16Sint
            | Self::R32g32Typeless
            | Self::R32g32Float
            | Self::R32g32Uint
            | Self::R32g32Sint
            | Self::R32g8x24Typeless
            | Self::D32FloatS8x24Uint
            | Self::R32FloatX8x24Typeless
            | Self::X32TypelessG8x24Uint
            | Self::Y416
            | Self::Y210
            | Self::Y216 => 64,
            Self::R10g10b10a2Typeless
            | Self::R10g10b10a2Unorm
            | Self::R10g10b10a2Uint
            | Self::R11g11b10Float
            | Self::R8g8b8a8Typeless
            | Self::R8g8b8a8Unorm
            | Self::R8g8b8a8UnormSrgb
            | Self::R8g8b8a8Uint
            | Self::R8g8b8a8Snorm
            | Self::R8g8b8a8Sint
            | Self::R16g16Typeless
            | Self::R16g16Float
            | Self::R16g16Unorm
            | Self::R16g16Uint
            | Self::R16g16Snorm
            | Self::R16g16Sint
            | Self::R32Typeless
            | Self::D32Float
            | Self::R32Float
            | Self::R32Uint
            | Self::R32Sint
            | Self::R24g8Typeless
            | Self::D24UnormS8Uint
            | Self::R24UnormX8Typeless
            | Self::X24TypelessG8Uint
            | Self::R9g9b9e5Sharedexp
            | Self::R8g8B8g8Unorm
            | Self::G8r8G8b8Unorm
            | Self::B8g8r8a8Unorm
            | Self::B8g8r8x8Unorm
            | Self::R10g10b10XrBiasA2Unorm
            | Self::B8g8r8a8Typeless
            | Self::B8g8r8a8UnormSrgb
            | Self::B8g8r8x8Typeless
            | Self::B8g8r8x8UnormSrgb
            | Self::Ayuv
            | Self::Y410
            | Self::Yuy2 => 32,
            Self::P010 | Self::P016 => 24,
            Self::R8g8Typeless
            | Self::R8g8Unorm
            | Self::R8g8Uint
            | Self::R8g8Snorm
            | Self::R8g8Sint
            | Self::R16Typeless
            | Self::R16Float
            | Self::D16Unorm
            | Self::R16Unorm
            | Self::R16Uint
            | Self::R16Snorm
            | Self::R16Sint
            | Self::B5g6r5Unorm
            | Self::B5g5r5a1Unorm
            | Self::A8p8
            | Self::B4g4r4a4Unorm => 16,
            Self::Nv12 | Self::Opaque420 | Self::Nv11 => 12,
            Self::R8Typeless
            | Self::R8Unorm
            | Self::R8Uint
            | Self::R8Snorm
            | Self::R8Sint
            | Self::A8Unorm
            | Self::Ai44
            | Self::Ia44
            | Self::P8 => 8,
            Self::R1Unorm => 1,
            Self::Bc1Typeless
            | Self::Bc1Unorm
            | Self::Bc1UnormSrgb
            | Self::Bc4Typeless
            | Self::Bc4Unorm
            | Self::Bc4Snorm => 4,
            Self::Bc2Typeless
            | Self::Bc2Unorm
            | Self::Bc2UnormSrgb
            | Self::Bc3Typeless
            | Self::Bc3Unorm
            | Self::Bc3UnormSrgb
            | Self::Bc5Typeless
            | Self::Bc5Unorm
            | Self::Bc5Snorm
            | Self::Bc6hTypeless
            | Self::Bc6hUf16
            | Self::Bc6hSf16
            | Self::Bc7Typeless
            | Self::Bc7Unorm
            | Self::Bc7UnormSrgb => 8,
            u => panic!("{u:?}"),
        }
    }

    pub fn calculate_pitch(&self, width: u32, height: u32) -> (usize, usize) {
        match self {
            Self::Bc1Typeless
            | Self::Bc1Unorm
            | Self::Bc1UnormSrgb
            | Self::Bc4Typeless
            | Self::Bc4Unorm
            | Self::Bc4Snorm => {
                let nbw = ((width as i64 + 3) / 4).clamp(1, i64::MAX) as usize;
                let nbh = ((height as i64 + 3) / 4).clamp(1, i64::MAX) as usize;

                let pitch = nbw * 8;
                (pitch, pitch * nbh)
            }
            Self::Bc2Typeless
            | Self::Bc2Unorm
            | Self::Bc2UnormSrgb
            | Self::Bc3Typeless
            | Self::Bc3Unorm
            | Self::Bc3UnormSrgb
            | Self::Bc5Typeless
            | Self::Bc5Unorm
            | Self::Bc5Snorm
            | Self::Bc6hTypeless
            | Self::Bc6hUf16
            | Self::Bc6hSf16
            | Self::Bc7Typeless
            | Self::Bc7Unorm
            | Self::Bc7UnormSrgb => {
                let nbw = ((width as i64 + 3) / 4).clamp(1, i64::MAX) as usize;
                let nbh = ((height as i64 + 3) / 4).clamp(1, i64::MAX) as usize;

                let pitch = nbw * 16;
                (pitch, pitch * nbh)
            }
            _ => {
                let pitch = (width * self.bpp()).div_ceil(8) as usize;
                (pitch, height as usize * pitch)
            }
        }
    }

    pub fn buffer_element_count(&self, size_in_bytes: u64) -> u64 {
        (size_in_bytes * 8).div_ceil(self.bpp() as u64)
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
            Ok(unsafe { std::mem::transmute::<u32, Self>(value) })
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
