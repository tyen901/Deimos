use glam::{Quat, Vec4};
use tiger_parse::tiger_type;

use crate::tfx::geometry::AxisAlignedBBox;

#[tiger_type(id = 0x8FFFFFFF)]
#[derive(Clone, Debug, Copy)]
pub struct SRotationTranslation {
    pub rotation: Quat,
    pub translation: Vec4,
}

impl SRotationTranslation {
    pub const IDENTITY: Self = Self {
        rotation: Quat::IDENTITY,
        translation: Vec4::ZERO,
    };
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x8080A9EA, size = 0x18)]
pub struct SOcclusionBounds {
    pub file_size: u64,
    pub bounds: Vec<SObjectOcclusionBounds>,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x8080A9EC, size = 0x30)]
pub struct SObjectOcclusionBounds {
    pub bb: AxisAlignedBBox,
    pub unk20: [u32; 4],
}
