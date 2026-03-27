use std::arch::x86_64::{_mm_rsqrt_ps, _mm_sqrt_ps};

use glam::{Vec3, Vec4};

pub trait Vec3Ext {
    // Copies the given vector with the vertical (Z) component set to 0
    fn flatten(&self) -> Vec3;
}

impl Vec3Ext for Vec3 {
    fn flatten(&self) -> Vec3 {
        Self::new(self.x, self.y, 0.0)
    }
}

pub trait Vec4Ext {
    fn sqrt(&self) -> Vec4;
    fn rsqrt(&self) -> Vec4;
}

impl Vec4Ext for Vec4 {
    fn sqrt(&self) -> Vec4 {
        unsafe { Self::from(_mm_sqrt_ps((*self).into())) }
    }

    fn rsqrt(&self) -> Vec4 {
        unsafe { Self::from(_mm_rsqrt_ps((*self).into())) }
    }
}

pub trait FloatExt: Sized {
    fn remap(self, in_start: f32, in_end: f32, out_start: f32, out_end: f32) -> f32;
    fn remap_clamped(self, in_start: f32, in_end: f32, out_start: f32, out_end: f32) -> f32 {
        let v = self.remap(in_start, in_end, out_start, out_end);
        if out_start < out_end {
            v.clamp(out_start, out_end)
        } else {
            v.clamp(out_end, out_start)
        }
    }
}

impl FloatExt for f32 {
    fn remap(self, in_start: f32, in_end: f32, out_start: f32, out_end: f32) -> f32 {
        out_start + (self - in_start) * (out_end - out_start) / (in_end - in_start)
    }
}
