use deimos_data::tfx::geometry::AxisAlignedBBox;
use glam::Vec3;

use crate::tfx::externs::View;

pub mod frustum;

pub struct ViewVisibility {
    pub position: glam::Vec3,
    pub culling_frustum: frustum::Frustum,
    pub world_to_projective: glam::Mat4,
}

impl ViewVisibility {
    pub fn is_visible(&self, aabb: &AxisAlignedBBox) -> bool {
        if aabb.contains_point(self.position) {
            return true;
        }

        if !self.culling_frustum.aabb_intersecting(aabb) {
            return false;
        }

        // Project the AABB corners to check how big they appear on screen
        let corners = aabb.points();
        let mut min_ndc = Vec3::splat(f32::MAX);
        let mut max_ndc = Vec3::splat(f32::MIN);
        for corner in &corners {
            let world_pos = corner.extend(1.0);
            let clip_pos = self.world_to_projective * world_pos;
            let ndc_pos = clip_pos.truncate() / clip_pos.w;

            min_ndc = min_ndc.min(ndc_pos);
            max_ndc = max_ndc.max(ndc_pos);
        }

        // If the projected size is too small, consider it not visible
        let ndc_size = max_ndc - min_ndc;
        let screen_size_threshold = 0.01; // Adjust this threshold as needed
        if ndc_size.x < screen_size_threshold && ndc_size.y < screen_size_threshold {
            return false;
        }

        true
    }
}
