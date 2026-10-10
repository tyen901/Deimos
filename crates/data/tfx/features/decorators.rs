use glam::Vec4;
use tiger_parse::tiger_type;
use tiger_pkg::TagHash;

use crate::{
    tag::{OptionalTagRef, TagRef},
    tfx::{common::SOcclusionBounds, geometry::AxisAlignedBBox},
};

#[derive(Clone, Debug)]
#[tiger_type(id = 0x8080857D)]
pub struct SDecorator {
    pub file_size: u64,
    pub unk8: Vec<TagRef<SUnk8080717E>>,
    pub unk18: Vec<u32>,
    pub unk28: Vec<u32>,
    pub unk38: Vec<u32>,
    pub unk48: TagRef<SUnk80807170>,
    pub unk4c: TagRef<SOcclusionBounds>,
    pub unk50: Vec<u32>,
    pub unk60: [u32; 4],
    pub bounds: AxisAlignedBBox,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x8080858B)]
pub struct SUnk80807170 {
    pub file_size: u64,
    pub unk8: u32,
    pub unkc: TagHash, // Vertex buffer data? (as opposed to a header)
    pub unk10: u32,
    pub unk14: TagRef<SUnk8080716B>,
    pub instance_buffer: TagHash,
    pub instance_data: TagRef<SDecoratorInstanceData>,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x8080858E)]
pub struct SDecoratorInstanceData {
    pub file_size: u64,
    pub data: Vec<SDecoratorInstanceElement>,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80808590)]
pub struct SDecoratorInstanceElement {
    /// Normalized position
    pub position: [u16; 3],
    /// Rotation represented as an 8-bit quaternion
    pub rotation: [u8; 4],
    /// RGBA color
    pub color: [u8; 4],
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80808586)]
pub struct SUnk8080716B {
    pub instances_scale: Vec4,
    pub instances_offset: Vec4,
    pub unk20: Vec4,
    pub unk30: Vec4,
    pub unk40: Vec4,
    pub unk50: Vec4,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80808599)]
pub struct SUnk8080717E {
    pub file_size: u64,
    pub entity_model: TagHash,
    pub unk8: u32,
    pub bounds: AxisAlignedBBox,
    pub unk10: TagHash,
    pub unk14: OptionalTagRef<SUnk80807184>,
    pub unk18: Vec<f32>,
    pub unk28: Vec<bool>,
    pub unk38: Vec<f32>,
    // ...
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80807184)]
pub struct SUnk80807184 {
    pub file_size: u64,
    pub unk8: Vec<SUnk80807186>,
}

#[derive(Clone, Debug)]
#[tiger_type(id = 0x80807186)]
pub struct SUnk80807186 {
    pub unk0: [Vec4; 5],
}

/// GPU instance wire format consumed by the retail decorator vertex programs.
/// This is distinct from the CPU-side SDecoratorInstanceElement schema.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DecoratorGpuInstance {
    pub position_scale: [i16; 4],
    pub rotation: [u8; 4],
    pub variation: [u8; 4],
}
impl DecoratorGpuInstance {
    pub const SIZE: usize = std::mem::size_of::<Self>();
    pub fn from_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), Self::SIZE);
        let rotation = std::mem::offset_of!(Self, rotation);
        let variation = std::mem::offset_of!(Self, variation);
        Self {
            position_scale: std::array::from_fn(|i| {
                let lane = i * std::mem::size_of::<i16>();
                i16::from_le_bytes([bytes[lane], bytes[lane + 1]])
            }),
            rotation: bytes[rotation..variation].try_into().unwrap(),
            variation: bytes[variation..Self::SIZE].try_into().unwrap(),
        }
    }
    pub fn position_scale(&self, constants: &SUnk8080716B) -> Vec4 {
        Vec4::from_array(
            self.position_scale
                .map(|lane| (f32::from(lane) / f32::from(i16::MAX)).max(-1.0)),
        ) * constants.instances_scale
            + constants.instances_offset
    }
    pub fn rotation(&self, constants: &SUnk8080716B) -> Vec4 {
        Vec4::from_array(
            self.rotation
                .map(|lane| f32::from(lane) / f32::from(u8::MAX)),
        ) * constants.unk20
            + constants.unk30
    }
}
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecoratorQuality {
    High,
}
