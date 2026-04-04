use deimos_data::tfx::geometry::AxisAlignedBBox;
use glam::Vec3;

pub mod bvh;
pub mod frustum;

pub struct ViewVisibility {
    /// If false, visibility checks will always return true.
    pub enabled: bool,
    pub position: glam::Vec3,
    pub far_plane: f32,
    pub culling_frustum: frustum::Frustum,
    pub world_to_projective: glam::Mat4,
    pub occlusion_buffer: Option<umbra::OcclusionBuffer>,
}

impl ViewVisibility {
    #[profiling::function]
    pub fn is_visible_quick(&self, aabb: &AxisAlignedBBox) -> bool {
        if !self.enabled {
            return true;
        }

        if let Some(occlusion_buffer) = &self.occlusion_buffer
            && !occlusion_buffer.is_aabb_visible(
                aabb.min.truncate().to_array(),
                aabb.max.truncate().to_array(),
            )
        {
            return false;
        }

        if aabb.contains_point(self.position) {
            return true;
        }

        if !self.culling_frustum.aabb_intersecting(aabb) {
            return false;
        }

        true
    }

    #[profiling::function]
    pub fn is_visible(&self, aabb: &AxisAlignedBBox) -> bool {
        if !self.enabled {
            return true;
        }

        if !self.is_visible_quick(aabb) {
            return false;
        }

        // Project the AABB corners to check how big they appear on screen
        let corners = aabb.points();
        let mut min_ndc = Vec3::splat(f32::MAX);
        let mut max_ndc = Vec3::splat(f32::MIN);
        for corner in &corners {
            let world_pos = corner.extend(1.0);
            let clip_pos = self.world_to_projective * world_pos;
            if clip_pos.w <= 0.0 {
                return true;
            }
            let ndc_pos = clip_pos.truncate() / clip_pos.w;

            min_ndc = min_ndc.min(ndc_pos);
            max_ndc = max_ndc.max(ndc_pos);
        }

        // If the projected size is too small, consider it not visible
        let ndc_size = max_ndc - min_ndc;
        let screen_size_threshold = 0.02; // Adjust this threshold as needed
        if ndc_size.x < screen_size_threshold && ndc_size.y < screen_size_threshold {
            return false;
        }

        true
    }
}
