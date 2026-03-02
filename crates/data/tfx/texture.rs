use tiger_parse::tiger_type;
use tiger_parse::TigerReadable;
use tiger_pkg::TagHash;

#[derive(Debug)]
#[tiger_type(etype = 32, size = 0x40)]
pub struct STextureHeader {
    pub data_size: u32,
    pub format: DxgiFormat,
    pub _unk8: u32,

    #[tiger(offset = 0x20)]
    pub cafe: u16,

    pub width: u16,
    pub height: u16,
    pub depth: u16,
    pub array_size: u16,
    pub tile_count: u16,

    pub unk2c: u8,
    #[deprecated(note = "Value is wrong. Use `mip_count` method to calculate mip count")]
    pub mip_count_broken: u8,
    pub unk2e: [u8; 10],
    pub unk38: u32,

    /// Optional
    pub large_buffer: TagHash,
}

impl STextureHeader {
    pub fn mip_count(&self) -> u16 {
        let smallest_dim = match self.dimension() {
            d3d12::ResourceDimension::Buffer | d3d12::ResourceDimension::Texture1D => self.width,
            d3d12::ResourceDimension::Texture2D => self.width.min(self.height),
            d3d12::ResourceDimension::Texture3D => self.width.min(self.height).min(self.depth),
        };

        let calculated_mip_count = ((smallest_dim as f32).log2().ceil() as u16).max(1);

        // This line is the exception to the rule, since mip_count_broken is 1 if the texture has no mips
        // calculated_mip_count is more or less used to correct the broken mip count
        #[allow(deprecated)]
        calculated_mip_count.min(self.mip_count_broken as u16)
    }

    pub const fn dimension(&self) -> d3d12::ResourceDimension {
        if self.depth > 1 {
            d3d12::ResourceDimension::Texture3D
        } else if self.height > 1 {
            d3d12::ResourceDimension::Texture2D
        } else {
            d3d12::ResourceDimension::Texture1D
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DxgiFormat(d3d12::Format);

impl TigerReadable for DxgiFormat {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let v = u32::read_ds_endian(reader, endian)?;
        Self::try_from(v).map_err(|_e| tiger_parse::Error::EnumVariantOutOfRange(v as usize))
    }

    const SIZE: usize = 4;
}

impl From<DxgiFormat> for u32 {
    fn from(val: DxgiFormat) -> Self {
        val.0 as Self
    }
}

impl From<DxgiFormat> for d3d12::Format {
    fn from(val: DxgiFormat) -> Self {
        val.0
    }
}

impl From<d3d12::Format> for DxgiFormat {
    fn from(val: d3d12::Format) -> Self {
        Self(val)
    }
}

impl TryFrom<u32> for DxgiFormat {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match d3d12::Format::try_from(value) {
            Ok(o) => Ok(Self(o)),
            Err(_) => Err(anyhow::anyhow!("DXGI format out of range")),
        }
    }
}

#[allow(unused)]
impl DxgiFormat {
    #[allow(clippy::match_same_arms)]
    pub fn bpp(&self) -> u32 {
        match self.0 {
            d3d12::Format::R32g32b32a32Typeless
            | d3d12::Format::R32g32b32a32Float
            | d3d12::Format::R32g32b32a32Uint
            | d3d12::Format::R32g32b32a32Sint => 128,
            d3d12::Format::R32g32b32Typeless
            | d3d12::Format::R32g32b32Float
            | d3d12::Format::R32g32b32Uint
            | d3d12::Format::R32g32b32Sint => 96,
            d3d12::Format::R16g16b16a16Typeless
            | d3d12::Format::R16g16b16a16Float
            | d3d12::Format::R16g16b16a16Unorm
            | d3d12::Format::R16g16b16a16Uint
            | d3d12::Format::R16g16b16a16Snorm
            | d3d12::Format::R16g16b16a16Sint
            | d3d12::Format::R32g32Typeless
            | d3d12::Format::R32g32Float
            | d3d12::Format::R32g32Uint
            | d3d12::Format::R32g32Sint
            | d3d12::Format::R32g8x24Typeless
            | d3d12::Format::D32FloatS8x24Uint
            | d3d12::Format::R32FloatX8x24Typeless
            | d3d12::Format::X32TypelessG8x24Uint
            | d3d12::Format::Y416
            | d3d12::Format::Y210
            | d3d12::Format::Y216 => 64,
            d3d12::Format::R10g10b10a2Typeless
            | d3d12::Format::R10g10b10a2Unorm
            | d3d12::Format::R10g10b10a2Uint
            | d3d12::Format::R11g11b10Float
            | d3d12::Format::R8g8b8a8Typeless
            | d3d12::Format::R8g8b8a8Unorm
            | d3d12::Format::R8g8b8a8UnormSrgb
            | d3d12::Format::R8g8b8a8Uint
            | d3d12::Format::R8g8b8a8Snorm
            | d3d12::Format::R8g8b8a8Sint
            | d3d12::Format::R16g16Typeless
            | d3d12::Format::R16g16Float
            | d3d12::Format::R16g16Unorm
            | d3d12::Format::R16g16Uint
            | d3d12::Format::R16g16Snorm
            | d3d12::Format::R16g16Sint
            | d3d12::Format::R32Typeless
            | d3d12::Format::D32Float
            | d3d12::Format::R32Float
            | d3d12::Format::R32Uint
            | d3d12::Format::R32Sint
            | d3d12::Format::R24g8Typeless
            | d3d12::Format::D24UnormS8Uint
            | d3d12::Format::R24UnormX8Typeless
            | d3d12::Format::X24TypelessG8Uint
            | d3d12::Format::R9g9b9e5Sharedexp
            | d3d12::Format::R8g8B8g8Unorm
            | d3d12::Format::G8r8G8b8Unorm
            | d3d12::Format::B8g8r8a8Unorm
            | d3d12::Format::B8g8r8x8Unorm
            | d3d12::Format::R10g10b10XrBiasA2Unorm
            | d3d12::Format::B8g8r8a8Typeless
            | d3d12::Format::B8g8r8a8UnormSrgb
            | d3d12::Format::B8g8r8x8Typeless
            | d3d12::Format::B8g8r8x8UnormSrgb
            | d3d12::Format::Ayuv
            | d3d12::Format::Y410
            | d3d12::Format::Yuy2 => 32,
            d3d12::Format::P010 | d3d12::Format::P016 => 24,
            d3d12::Format::R8g8Typeless
            | d3d12::Format::R8g8Unorm
            | d3d12::Format::R8g8Uint
            | d3d12::Format::R8g8Snorm
            | d3d12::Format::R8g8Sint
            | d3d12::Format::R16Typeless
            | d3d12::Format::R16Float
            | d3d12::Format::D16Unorm
            | d3d12::Format::R16Unorm
            | d3d12::Format::R16Uint
            | d3d12::Format::R16Snorm
            | d3d12::Format::R16Sint
            | d3d12::Format::B5g6r5Unorm
            | d3d12::Format::B5g5r5a1Unorm
            | d3d12::Format::A8p8
            | d3d12::Format::B4g4r4a4Unorm => 16,
            d3d12::Format::Nv12 | d3d12::Format::Opaque420 | d3d12::Format::Nv11 => 12,
            d3d12::Format::R8Typeless
            | d3d12::Format::R8Unorm
            | d3d12::Format::R8Uint
            | d3d12::Format::R8Snorm
            | d3d12::Format::R8Sint
            | d3d12::Format::A8Unorm
            | d3d12::Format::Ai44
            | d3d12::Format::Ia44
            | d3d12::Format::P8 => 8,
            d3d12::Format::R1Unorm => 1,
            d3d12::Format::Bc1Typeless
            | d3d12::Format::Bc1Unorm
            | d3d12::Format::Bc1UnormSrgb
            | d3d12::Format::Bc4Typeless
            | d3d12::Format::Bc4Unorm
            | d3d12::Format::Bc4Snorm => 4,
            d3d12::Format::Bc2Typeless
            | d3d12::Format::Bc2Unorm
            | d3d12::Format::Bc2UnormSrgb
            | d3d12::Format::Bc3Typeless
            | d3d12::Format::Bc3Unorm
            | d3d12::Format::Bc3UnormSrgb
            | d3d12::Format::Bc5Typeless
            | d3d12::Format::Bc5Unorm
            | d3d12::Format::Bc5Snorm
            | d3d12::Format::Bc6hTypeless
            | d3d12::Format::Bc6hUf16
            | d3d12::Format::Bc6hSf16
            | d3d12::Format::Bc7Typeless
            | d3d12::Format::Bc7Unorm
            | d3d12::Format::Bc7UnormSrgb => 8,
            u => panic!("{u:?}"),
        }
    }

    pub const fn is_srgb(&self) -> bool {
        matches!(
            self.0,
            d3d12::Format::R8g8b8a8UnormSrgb
                | d3d12::Format::Bc1UnormSrgb
                | d3d12::Format::Bc2UnormSrgb
                | d3d12::Format::Bc3UnormSrgb
                | d3d12::Format::B8g8r8a8UnormSrgb
                | d3d12::Format::B8g8r8x8UnormSrgb
                | d3d12::Format::Bc7UnormSrgb
        )
    }

    pub const fn is_compressed(&self) -> bool {
        matches!(
            self.0,
            d3d12::Format::Bc1Typeless
                | d3d12::Format::Bc1Unorm
                | d3d12::Format::Bc1UnormSrgb
                | d3d12::Format::Bc4Typeless
                | d3d12::Format::Bc4Unorm
                | d3d12::Format::Bc4Snorm
                | d3d12::Format::Bc2Typeless
                | d3d12::Format::Bc2Unorm
                | d3d12::Format::Bc2UnormSrgb
                | d3d12::Format::Bc3Typeless
                | d3d12::Format::Bc3Unorm
                | d3d12::Format::Bc3UnormSrgb
                | d3d12::Format::Bc5Typeless
                | d3d12::Format::Bc5Unorm
                | d3d12::Format::Bc5Snorm
                | d3d12::Format::Bc6hTypeless
                | d3d12::Format::Bc6hUf16
                | d3d12::Format::Bc6hSf16
                | d3d12::Format::Bc7Typeless
                | d3d12::Format::Bc7Unorm
                | d3d12::Format::Bc7UnormSrgb
        )
    }

    pub fn calculate_pitch(&self, width: u32, height: u32) -> (usize, usize) {
        match self.0 {
            d3d12::Format::Bc1Typeless
            | d3d12::Format::Bc1Unorm
            | d3d12::Format::Bc1UnormSrgb
            | d3d12::Format::Bc4Typeless
            | d3d12::Format::Bc4Unorm
            | d3d12::Format::Bc4Snorm => {
                let nbw = ((width as i64 + 3) / 4).clamp(1, i64::MAX) as usize;
                let nbh = ((height as i64 + 3) / 4).clamp(1, i64::MAX) as usize;

                let pitch = nbw * 8;
                (pitch, pitch * nbh)
            }
            d3d12::Format::Bc2Typeless
            | d3d12::Format::Bc2Unorm
            | d3d12::Format::Bc2UnormSrgb
            | d3d12::Format::Bc3Typeless
            | d3d12::Format::Bc3Unorm
            | d3d12::Format::Bc3UnormSrgb
            | d3d12::Format::Bc5Typeless
            | d3d12::Format::Bc5Unorm
            | d3d12::Format::Bc5Snorm
            | d3d12::Format::Bc6hTypeless
            | d3d12::Format::Bc6hUf16
            | d3d12::Format::Bc6hSf16
            | d3d12::Format::Bc7Typeless
            | d3d12::Format::Bc7Unorm
            | d3d12::Format::Bc7UnormSrgb => {
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
}
