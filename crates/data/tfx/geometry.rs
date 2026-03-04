use std::iter::Sum;

use glam::{vec3, Vec3, Vec4, Vec4Swizzles};
use tiger_parse::tiger_type;

#[tiger_type]
#[derive(Debug, Clone)]
pub struct AxisAlignedBBox {
    pub min: Vec4,
    pub max: Vec4,
}

impl AxisAlignedBBox {
    pub const NONE: Self = Self {
        min: Vec4::new(f32::MAX, f32::MAX, f32::MAX, f32::MAX),
        max: Vec4::new(f32::MIN, f32::MIN, f32::MIN, f32::MIN),
    };

    pub const EVERYTHING: Self = Self {
        min: Vec4::new(f32::MIN, f32::MIN, f32::MIN, f32::MIN),
        max: Vec4::new(f32::MAX, f32::MAX, f32::MAX, f32::MAX),
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

    pub fn from_points(points: &[Vec3]) -> Self {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);

        for &point in points {
            min = min.min(point);
            max = max.max(point);
        }

        Self {
            min: min.extend(1.0),
            max: max.extend(1.0),
        }
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

    pub fn transformed(&self, transform: glam::Mat4) -> Self {
        let points = self.points();
        let transformed_points: Vec<Vec3> = points
            .iter()
            .map(|&point| transform.transform_point3(point))
            .collect();
        Self::from_points(&transformed_points)
    }

    pub fn extents(&self) -> Vec3 {
        (self.max - self.min).xyz()
    }

    pub fn centroid(&self) -> Vec3 {
        ((self.min + self.max) / 2.0).xyz()
    }

    pub fn longest_axis(&self) -> usize {
        let d = self.max - self.min;
        if d.x >= d.y && d.x >= d.z {
            0
        } else if d.y >= d.z {
            1
        } else {
            2
        }
    }

    pub fn radius(&self) -> f32 {
        self.extents().length() / 2.0
    }

    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub fn sphere(&self) -> SphereBounds {
        SphereBounds {
            center: self.centroid(),
            radius: self.radius(),
        }
    }

    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
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

#[derive(Debug, Default, Clone, Copy)]
pub struct SphereBounds {
    pub center: Vec3,
    pub radius: f32,
}

impl SphereBounds {
    pub const INFINITE: Self = Self {
        center: Vec3::ZERO,
        radius: f32::INFINITY,
    };
}
