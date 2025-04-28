use std::{sync::Arc, time::Instant};

use d3d11::{dxgi, ShaderResourceView, Texture2D, Texture2dDesc};
use deimos_render::{
    camera::Camera,
    gpu::command_list::CommandList,
    object::{RenderObject, RenderObjectHandle},
    tfx::{packet::CompactTransform, view::View},
    Gpu, Renderer,
};
use egui::{load::SizedTexture, vec2, Response, Sense, Ui, Vec2};
use glam::{Mat4, Quat, Vec3};

pub struct Scene {
    renderer: Arc<Renderer>,
    camera: Camera,
    view: View,
    last_frame_time: Instant,

    controller: CameraController,

    static_render_objects: Vec<RenderObjectHandle>,
    dynamic_render_objects: Vec<(RenderObjectHandle, Vec3)>,

    surface: d3d11::Texture2D,
    surface_srv: d3d11::ShaderResourceView,
}

impl Scene {
    pub fn new(renderer: Arc<Renderer>, camera: Camera) -> anyhow::Result<Self> {
        let (surface, surface_srv) = Self::create_surface(&renderer.gpu, (128, 128))?;

        Ok(Self {
            view: View::new(&renderer.gpu, (128, 128))?,
            renderer,
            camera,
            controller: CameraController::new_orbit(Vec3::ZERO, 3.5),
            static_render_objects: Vec::new(),
            dynamic_render_objects: Vec::new(),
            surface,
            surface_srv,
            last_frame_time: Instant::now(),
        })
    }

    pub fn with_controller(mut self, controller: CameraController) -> Self {
        self.controller = controller;
        self
    }

    fn create_surface(
        gpu: &Gpu,
        resolution: (u32, u32),
    ) -> anyhow::Result<(Texture2D, ShaderResourceView)> {
        let texture = gpu.create_texture2d(
            &Texture2dDesc::builder()
                .width(resolution.0)
                .height(resolution.1)
                .mip_levels(1)
                .format(dxgi::Format::R8g8b8a8UnormSrgb)
                .bind_flags(d3d11::BindFlags::SHADER_RESOURCE)
                .build(),
            None,
        )?;

        let srv = gpu.create_shader_resource_view(&texture, None)?;

        Ok((texture, srv))
    }

    pub fn add_static_object(&mut self, object: RenderObject) {
        self.static_render_objects
            .push(self.renderer.add_object(object));
    }

    pub fn add_dynamic_object(&mut self, object: RenderObject, pos: Vec3) {
        self.dynamic_render_objects
            .push((self.renderer.add_object(object), pos));
    }

    pub fn clear(&mut self) {
        self.static_render_objects
            .drain(..)
            .for_each(|h| self.renderer.remove_object(h));
        self.dynamic_render_objects
            .drain(..)
            .for_each(|(h, _)| self.renderer.remove_object(h));
    }

    pub fn show(&mut self, ui: &mut Ui, size: Vec2, egui_d3d11: &mut egui_d3d11::D3D11Renderer) {
        let r = ui
            .image(SizedTexture {
                id: egui_d3d11.textures_mut().allocate_dx_temporary(
                    self.surface_srv.clone(),
                    Some(egui::TextureFilter::Linear),
                ),
                size,
            })
            .interact(Sense::CLICK | Sense::DRAG | Sense::HOVER);

        if !ui.is_rect_visible(r.rect) {
            return;
        }

        let now = Instant::now();
        let delta_time = (now - self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        ui.painter_at(r.rect).text(
            r.rect.right_top() + Vec2::splat(1.0),
            egui::Align2::RIGHT_TOP,
            format!("{} ", (1. / delta_time).round()),
            egui::FontId::monospace(16.0),
            egui::Color32::BLACK,
        );

        ui.painter_at(r.rect).text(
            r.rect.right_top(),
            egui::Align2::RIGHT_TOP,
            format!("{} ", (1. / delta_time).round()),
            egui::FontId::monospace(16.0),
            egui::Color32::GREEN,
        );

        let size_pixels = size * ui.ctx().pixels_per_point();
        let resolution = (size_pixels.x as u32, size_pixels.y as u32);
        if resolution != self.surface.get_desc().resolution() {
            let (texture, srv) = Self::create_surface(&self.renderer.gpu, resolution)
                .expect("Failed to resize scene surface");
            self.surface = texture;
            self.surface_srv = srv;
        }

        self.camera.aspect_ratio = resolution.0 as f32 / resolution.1 as f32;
        self.controller.update(&mut self.camera, ui, &r, delta_time);
        self.camera.update();
        let camera_to_projective = self.camera.projection_matrix(self.camera.aspect_ratio);
        let world_to_camera = self.camera.view_matrix();
        self.view
            .update(world_to_camera, camera_to_projective, resolution);

        self.render(delta_time);
    }

    fn render(&mut self, delta_time: f32) {
        self.renderer.frame_packet.write().reset();

        {
            let mut fp = self.renderer.frame_packet.write();
            for t in &self.static_render_objects {
                fp.push_static_render_object(*t);
            }
            for (t, pos) in &self.dynamic_render_objects {
                fp.push_dynamic_render_object(
                    *t,
                    CompactTransform::from_mat4(Mat4::from_translation(*pos)),
                );
            }
        }

        let gpu = &self.renderer.gpu;
        let mut cmd = CommandList::from_device_context(gpu, gpu.context().clone());
        {
            profiling::scope!("prepare");

            cmd.clear_render_target_view(&gpu.acquire_rtv(), &[0.0, 0.0, 0.0, 1.0]);

            {
                profiling::scope!("visibility");
                self.renderer
                    .frame_packet
                    .write()
                    .frame_nodes
                    .retain(|node| {
                        if let Some(render_object) = self
                            .renderer
                            .objects
                            .write()
                            .get_mut(node.render_object_handle.into())
                        {
                            render_object.visibility_test(&self.camera)
                        } else {
                            true
                        }
                    });
            }

            for node in self.renderer.frame_packet.read().frame_nodes.iter() {
                if let Some(render_object) = self
                    .renderer
                    .objects
                    .write()
                    .get_mut(node.render_object_handle.into())
                {
                    render_object.extract_and_prepare(&self.renderer, &*node.data);
                } else if node.render_object_handle.is_valid() {
                    error!("Render object not found: {:?}", node.render_object_handle);
                }
            }

            // Sort nodes by distance
            self.renderer
                .frame_packet
                .write()
                .frame_nodes
                .sort_by(|n1, n2| {
                    n2.distance
                        .partial_cmp(&n1.distance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
        }

        self.renderer.submit_world(&mut cmd, &self.view, delta_time);
        cmd.copy_resource(
            &self.renderer.surfaces().get(self.view.output).texture,
            &self.surface,
        );
        // let cmd = self.draw_world(delta_time);
        // self.renderer.gpu.submit_command_list(cmd);

        // if self.show_debug_text
        // {
        //     let gpu = &self.renderer.gpu;
        //     let context = gpu.context();

        //     context.rasterizer_set_viewports(&[d3d11::Viewport::builder()
        //         .width(gpu.swapchain_resolution().0 as f32)
        //         .height(gpu.swapchain_resolution().1 as f32)
        //         .build()]);
        //     context.output_merger_set_render_targets(&[Some(gpu.acquire_rtv())], None);
        //     context.output_merger_set_depth_stencil_state(None, 0);
        //     context.rasterizer_set_state(None);
        //     self.renderer.debug_text.lock().draw(&self.renderer.gpu);
        // }
    }

    pub fn focus_on(&mut self, position: Vec3) {
        match &mut self.controller {
            CameraController::Orbit { target, .. } => {
                *target = position;
            }
            CameraController::FirstPerson { .. } => {
                self.camera.position = position;
            }
        }
    }
}

impl Drop for Scene {
    fn drop(&mut self) {
        for &h in &self.static_render_objects {
            self.renderer.remove_object(h);
        }
        for &(h, _) in &self.dynamic_render_objects {
            self.renderer.remove_object(h);
        }
    }
}

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
                if !response.dragged_by(egui::PointerButton::Primary) {
                    return;
                }

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

                let drag_delta = response.drag_delta();
                *yaw_pitch += (drag_delta / 10.0) * vec2(-1.0, 1.3);
                yaw_pitch.y = yaw_pitch.y.clamp(-89.0, 89.0);

                camera.rotation = Quat::from_rotation_z(yaw_pitch.x.to_radians())
                    * Quat::from_rotation_y(yaw_pitch.y.to_radians());
            }
        }
    }
}
