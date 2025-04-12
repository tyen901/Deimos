use destiny_pkg::TagHash;
use std::mem::transmute;
use tiger_parse::tiger_tag;
use tiger_parse::TigerReadable;

#[derive(Debug)]
#[tiger_tag(etype = 32, size = 0x28)]
pub struct STextureHeader {
    pub data_size: u32,
    pub format: DxgiFormat,
    pub _unk8: u32,

    #[tag(offset = 0x20)]
    pub cafe: u16,

    pub width: u16,
    pub height: u16,
    pub depth: u16,
    pub array_size: u16,
    pub tile_count: u16,

    pub unk2c: u8,
    pub mip_count: u8,
    pub unk2e: [u8; 10],
    pub unk38: u32,

    /// Optional
    pub large_buffer: TagHash,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DxgiFormat(d3d11::dxgi::Format);

impl TigerReadable for DxgiFormat {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        Ok(unsafe { transmute(u32::read_ds_endian(reader, endian)?) })
    }

    const SIZE: usize = 4;
}

impl From<DxgiFormat> for u32 {
    fn from(val: DxgiFormat) -> Self {
        unsafe { transmute(val) }
    }
}

impl TryFrom<u32> for DxgiFormat {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0..=115 | 130..=132 => unsafe { transmute(value) },
            e => return Err(anyhow::anyhow!("DXGI format is out of range ({e})")),
        })
    }
}
