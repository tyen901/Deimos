use deimos_render::{camera::Camera, Renderer};
use egui::{vec2, Response, Ui, Vec2};
use glam::{Quat, Vec3};

pub enum CameraController {
    Orbit {
        target: Vec3,
        distance: f32,
        yaw_pitch: Vec2,
    },
    FirstPerson {
        speed: f32,
        yaw_pitch: Vec2,
    },
}

impl CameraController {
    pub fn new_orbit(target: Vec3, distance: f32) -> Self {
        Self::Orbit {
            target,
            distance,
            yaw_pitch: Vec2::new(180.0, 0.0),
        }
    }

    pub fn new_first_person() -> Self {
        Self::FirstPerson {
            speed: 25.0,
            yaw_pitch: Vec2::ZERO,
        }
    }

    pub fn update(&mut self, camera: &mut Camera, ui: &Ui, response: &Response, delta_time: f32) {
        match self {
            Self::Orbit {
                target,
                distance,
                yaw_pitch,
            } => {
                if response.hovered() {
                    let scroll_delta = ui.input(|i| i.raw_scroll_delta);
                    *distance += -scroll_delta.y / 250.0;
                    *distance = distance.clamp(0.01, 1000.0);
                }
                let real_distance = 2.0f32.powf(*distance * 0.3) - 0.9;

                let drag_delta = response.drag_delta();
                // Rotate
                if response.dragged_by(egui::PointerButton::Primary) {
                    *yaw_pitch += (drag_delta / 5.0) * vec2(-1.0, 1.3);
                    yaw_pitch.y = yaw_pitch.y.clamp(-89.0, 89.0);
                }

                // Pan
                if response.dragged_by(egui::PointerButton::Middle) {
                    let delta_adjusted = (drag_delta / 250.0) * real_distance;
                    *target -= camera.right() * delta_adjusted.x;
                    *target += camera.up() * delta_adjusted.y;
                }

                if response.dragged() {
                    Renderer::instance()
                        .immediate
                        .lock()
                        .cross(*target, 0.15, 0xffffff);
                }

                camera.rotation = Quat::from_rotation_z(yaw_pitch.x.to_radians())
                    * Quat::from_rotation_y(yaw_pitch.y.to_radians());

                camera.position = *target - camera.forward() * real_distance;
            }
            Self::FirstPerson { speed, yaw_pitch } => {
                let mut movement = Vec3::ZERO;
                ui.input(|i| {
                    if i.key_down(egui::Key::W) {
                        movement += camera.forward();
                    }
                    if i.key_down(egui::Key::S) {
                        movement -= camera.forward();
                    }
                    if i.key_down(egui::Key::A) {
                        movement -= camera.right();
                    }
                    if i.key_down(egui::Key::D) {
                        movement += camera.right();
                    }
                    if i.key_down(egui::Key::Q) {
                        movement -= camera.up();
                    }
                    if i.key_down(egui::Key::E) {
                        movement += camera.up();
                    }
                    movement = movement.normalize_or(Vec3::ZERO);
                    if i.modifiers.ctrl {
                        movement /= 5.0;
                    }
                    if i.modifiers.shift {
                        movement *= 2.0;
                    }
                    if i.key_down(egui::Key::Space) {
                        movement *= 2.5;
                    }
                });
                camera.position += movement * delta_time * *speed;

                if response.dragged_by(egui::PointerButton::Primary)
                    | response.dragged_by(egui::PointerButton::Secondary)
                {
                    let drag_delta = response.drag_delta();
                    *yaw_pitch += (drag_delta / 10.0) * vec2(-1.0, 1.3);
                    yaw_pitch.y = yaw_pitch.y.clamp(-89.0, 89.0);

                    camera.rotation = Quat::from_rotation_z(yaw_pitch.x.to_radians())
                        * Quat::from_rotation_y(yaw_pitch.y.to_radians());
                }
            }
        }
    }
}
