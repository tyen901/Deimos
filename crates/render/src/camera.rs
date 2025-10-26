use glam::{Mat4, Quat, Vec3, Vec4};

use crate::visibility::frustum::Frustum;

// A simple camera controller (X forward, Z up)
pub struct Camera {
    pub position: Vec3,
    pub rotation: Quat,
    pub fov_y: f32,

    pub near: f32,
    pub far: f32,
    pub aspect_ratio: f32,
    pub frustum: Frustum,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            fov_y: 90.0,
            near: Self::NEAR,
            far: Self::FAR,
            aspect_ratio: 16. / 9.,
            frustum: Frustum::default(),
        }
    }
}

impl Camera {
    pub const NEAR: f32 = 0.05;
    pub const FAR: f32 = 50000.0;

    pub fn update(&mut self) {
        self.frustum = Frustum::from_camera(self);
    }

    pub fn view_matrix(&self) -> glam::Mat4 {
        glam::Mat4::look_at_rh(
            self.position,
            self.position + self.rotation.mul_vec3(Vec3::X),
            Vec3::Z,
        )
    }

    pub fn projection_matrix(&self, aspect_ratio: f32) -> glam::Mat4 {
        let f = 1.0 / f32::tan(0.5 * self.fov_y.to_radians());
        let far = (1. / self.far) * self.near;
        Mat4::from_cols(
            Vec4::new(f / aspect_ratio, 0.0, 0.0, 0.0),
            Vec4::new(0.0, f, 0.0, 0.0),
            Vec4::new(0.0, 0.0, far, -1.0),
            Vec4::new(0.0, 0.0, self.near, 0.0),
        )
    }

    pub fn projection_matrix_standard(&self) -> glam::Mat4 {
        Mat4::perspective_rh(
            self.fov_y.to_radians(),
            self.aspect_ratio,
            self.near,
            self.far,
        )
    }

    pub fn forward(&self) -> Vec3 {
        self.rotation.mul_vec3(Vec3::X)
    }

    pub fn right(&self) -> Vec3 {
        self.rotation.mul_vec3(-Vec3::Y)
    }

    pub fn up(&self) -> Vec3 {
        self.rotation.mul_vec3(Vec3::Z)
    }
}
