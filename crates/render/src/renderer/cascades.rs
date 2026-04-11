pub struct CascadeCalculator;

impl CascadeCalculator {
    pub const MAX_CASCADES: usize = 4;
    pub const MAX_FAR_PLANE: f32 = 500.0;

    pub fn get_depth_range(cascade_index: usize, far_plane: f32) -> Option<(f32, f32)> {
        let far_plane = far_plane.min(Self::MAX_FAR_PLANE);
        Some(match cascade_index {
            0 => (0.0, far_plane / 8.0),
            1 => (far_plane / 8.0, far_plane / 4.0),
            2 => (far_plane / 4.0, far_plane / 2.0),
            3 => (far_plane / 2.0, far_plane),
            _ => return None,
        })
    }
}
