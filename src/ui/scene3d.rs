use std::{sync::Arc, time::Instant};

use d3d11::{dxgi, ShaderResourceView, Texture2D, Texture2dDesc};
use deimos_render::{
    camera::Camera,
    gpu::command_list::CommandList,
    object::{RenderObject, RenderObjectHandle},
    tfx::view::View,
    Gpu, Renderer,
};
use egui::{load::SizedTexture, vec2, Response, Sense, Ui, Vec2};
use glam::Quat;

pub struct Scene {
    renderer: Arc<Renderer>,
    camera: Camera,
    camera_yaw_pitch: Vec2,
    view: View,
    last_frame_time: Instant,

    static_render_objects: Vec<RenderObjectHandle>,

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
            camera_yaw_pitch: Vec2::ZERO,
            static_render_objects: Vec::new(),
            surface,
            surface_srv,
            last_frame_time: Instant::now(),
        })
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

        let size_pixels = size * ui.ctx().pixels_per_point();
        let resolution = (size_pixels.x as u32, size_pixels.y as u32);
        if resolution != self.surface.get_desc().resolution() {
            let (texture, srv) = Self::create_surface(&self.renderer.gpu, resolution)
                .expect("Failed to resize scene surface");
            self.surface = texture;
            self.surface_srv = srv;
        }
        let now = Instant::now();
        let delta_time = (now - self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        self.update_camera_movement(r, delta_time);

        self.camera.aspect_ratio = resolution.0 as f32 / resolution.1 as f32;
        self.camera.update();
        let camera_to_projective = self.camera.projection_matrix(self.camera.aspect_ratio);
        let world_to_camera = self.camera.view_matrix();
        self.view
            .update(world_to_camera, camera_to_projective, resolution);

        self.render(delta_time);
    }

    fn update_camera_movement(&mut self, r: Response, _delta_time: f32) {
        // let mut movement = Vec3::ZERO;
        // if self.input.is_key_down(Keycode::W) {
        //     movement += self.camera.forward();
        // }
        // if self.input.is_key_down(Keycode::S) {
        //     movement -= self.camera.forward();
        // }
        // if self.input.is_key_down(Keycode::A) {
        //     movement -= self.camera.right();
        // }
        // if self.input.is_key_down(Keycode::D) {
        //     movement += self.camera.right();
        // }
        // if self.input.is_key_down(Keycode::Q) {
        //     movement -= self.camera.up();
        // }
        // if self.input.is_key_down(Keycode::E) {
        //     movement += self.camera.up();
        // }
        // movement = movement.normalize_or(Vec3::ZERO);
        // if self.input.is_key_down(Keycode::LCtrl) {
        //     movement /= 5.0;
        // }
        // if self.input.is_key_down(Keycode::LShift) {
        //     movement *= 2.0;
        // }
        // if self.input.is_key_down(Keycode::Space) {
        //     movement *= 2.5;
        // }

        // self.camera.position += movement * delta_time * 40.0;

        let drag_delta = r.drag_delta();
        self.camera_yaw_pitch += (drag_delta / 10.0) * vec2(-1.0, 1.3);
        self.camera_yaw_pitch.y = self.camera_yaw_pitch.y.clamp(-89.0, 89.0);

        self.camera.rotation = Quat::from_rotation_z(self.camera_yaw_pitch.x.to_radians())
            * Quat::from_rotation_y(self.camera_yaw_pitch.y.to_radians());

        self.camera.position = -self.camera.forward() * 100.0;
    }

    fn render(&mut self, delta_time: f32) {
        self.renderer.frame_packet.write().reset();

        {
            let mut fp = self.renderer.frame_packet.write();
            for t in &self.static_render_objects {
                fp.push_static_render_object(*t);
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
}

impl Drop for Scene {
    fn drop(&mut self) {
        for &h in &self.static_render_objects {
            self.renderer.remove_object(h);
        }
    }
}
