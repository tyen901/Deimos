use std::iter::Sum;

use glam::{vec3, Vec3, Vec4, Vec4Swizzles};
use tiger_parse::tiger_type;

#[derive(Debug, Clone)]
#[tiger_type(id = 0x8080A7F5, size = 0x18)]
pub struct SOcclusionBounds {
    pub file_size: u64,
    pub bounds: Vec<SObjectOcclusionBounds>,
}

#[derive(Debug, Clone)]
#[tiger_type(id = 0x8080A7F7, size = 0x30)]
pub struct SObjectOcclusionBounds {
    pub bb: AxisAlignedBBox,
    pub unk20: [u32; 4],
}

#[tiger_type]
#[derive(Debug, Clone)]
pub struct AxisAlignedBBox {
    pub min: Vec4,
    pub max: Vec4,
}

impl AxisAlignedBBox {
    const NONE: Self = Self {
        min: Vec4::new(f32::MAX, f32::MAX, f32::MAX, f32::MAX),
        max: Vec4::new(f32::MIN, f32::MIN, f32::MIN, f32::MIN),
    };

    pub fn from_center_extents(center: Vec3, extents: Vec3) -> Self {
        Self {
            min: vec3(
                center.x - extents.x / 2.0,
                center.y - extents.y / 2.0,
                center.z - extents.z / 2.0,
            )
            .extend(0.0),
            max: vec3(
                center.x + extents.x / 2.0,
                center.y + extents.y / 2.0,
                center.z + extents.z / 2.0,
            )
            .extend(0.0),
        }
    }

    pub fn extents(&self) -> Vec3 {
        (self.max - self.min).xyz()
    }

    pub fn center(&self) -> Vec3 {
        ((self.min + self.max) / 2.0).xyz()
    }

    pub fn radius(&self) -> f32 {
        self.extents().length() / 2.0
    }

    pub fn points(&self) -> [Vec3; 8] {
        [
            vec3(self.min.x, self.min.y, self.min.z),
            vec3(self.min.x, self.min.y, self.max.z),
            vec3(self.min.x, self.max.y, self.min.z),
            vec3(self.min.x, self.max.y, self.max.z),
            vec3(self.max.x, self.min.y, self.min.z),
            vec3(self.max.x, self.min.y, self.max.z),
            vec3(self.max.x, self.max.y, self.min.z),
            vec3(self.max.x, self.max.y, self.max.z),
        ]
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
}

impl Sum for AxisAlignedBBox {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Self>,
    {
        iter.fold(Self::NONE, |acc, bbox| acc.union(&bbox))
    }
}
