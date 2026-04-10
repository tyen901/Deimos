pub struct CascadeCalculator;

impl CascadeCalculator {
    pub const MAX_CASCADES: usize = 4;
    pub const FAR_PLANE: f32 = 500.0;

    pub fn get_depth_range(cascade_index: usize) -> Option<(f32, f32)> {
        Some(match cascade_index {
            0 => (0.0, Self::FAR_PLANE / 8.0),
            1 => (Self::FAR_PLANE / 8.0, Self::FAR_PLANE / 4.0),
            2 => (Self::FAR_PLANE / 4.0, Self::FAR_PLANE / 2.0),
            3 => (Self::FAR_PLANE / 2.0, Self::FAR_PLANE),
            _ => return None,
        })
    }
}
