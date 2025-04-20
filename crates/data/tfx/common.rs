use glam::{Vec3, Vec4, Vec4Swizzles};
use tiger_parse::tiger_tag;

#[tiger_tag]
#[derive(Debug, Clone)]
pub struct AxisAlignedBBox {
    pub min: Vec4,
    pub max: Vec4,
}

impl AxisAlignedBBox {
    pub fn extents(&self) -> Vec3 {
        (self.max - self.min).xyz()
    }

    pub fn center(&self) -> Vec3 {
        ((self.min + self.max) / 2.0).xyz()
    }
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x8080A7F5, size = 0x18)]
pub struct SOcclusionBounds {
    pub file_size: u64,
    pub bounds: Vec<SObjectOcclusionBounds>,
}

#[derive(Debug, Clone)]
#[tiger_tag(id = 0x8080A7F7, size = 0x30)]
pub struct SObjectOcclusionBounds {
    pub bb: AxisAlignedBBox,
    pub unk20: [u32; 4],
}
