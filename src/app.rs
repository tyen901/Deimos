use std::{f32, rc::Rc, sync::Arc, time::Instant};

use deimos_data::tfx::TfxFeatureRenderer;
use deimos_render::{
    camera::Camera,
    feature::static_geometry::{load_static_map, StaticMapTemp},
    gpu::{debug_text::DebugTextAlign, spinner::FullscreenSpinner},
    object::{RenderObject, RenderObjectHandle},
    tfx::packet::FrameNode,
    Gpu, Renderer,
};
use glam::{vec2, IVec2, Quat, Vec2, Vec3};
use sdl3::{keyboard::Keycode, video::Window};
use tiger_pkg::{package_manager, TagHash};

use crate::input::{MouseButton, MouseKeyboardState};

pub struct App {
    pub window: Rc<Window>,
    pub gpu: Arc<Gpu>,
    pub renderer: Arc<Renderer>,

    input: MouseKeyboardState,
    spinner: FullscreenSpinner,
    last_frame_time: Instant,
    start_time: Instant,
    camera: Camera,

    // map: StaticMapTemp,
    static_render_objects: Vec<RenderObjectHandle>,

    yaw_pitch: Vec2,
}

impl App {
    pub fn new(sdl: Rc<sdl3::Sdl>, window: Rc<Window>, map: TagHash) -> anyhow::Result<Self> {
        let gpu = Arc::new(Gpu::create(&window)?);
        let renderer = Arc::new(Renderer::new(gpu.clone(), window.size())?);
        Renderer::set_instance(renderer.clone());

        let map = load_static_map(map)?;

        let mut static_render_objects = Vec::new();
        for t in map.terrain {
            static_render_objects.push(renderer.add_object(RenderObject::new(
                TfxFeatureRenderer::TerrainPatch,
                Box::new(t),
                Box::new(()),
            )));
        }

        Ok(Self {
            input: MouseKeyboardState::new(sdl.clone(), window.clone()),
            spinner: FullscreenSpinner::create(&renderer.gpu)?,
            renderer,
            window,
            gpu,

            last_frame_time: Instant::now(),
            start_time: Instant::now(),
            camera: Camera::default(),
            // map,
            static_render_objects,
            yaw_pitch: Vec2::ZERO,
        })
    }

    pub fn handle_event(&mut self, event: sdl3::event::Event) {
        #[allow(clippy::single_match, clippy::collapsible_match)]
        match &event {
            sdl3::event::Event::Quit { .. } => {
                std::process::exit(0);
            }
            sdl3::event::Event::Window { win_event, .. } => match win_event {
                &sdl3::event::WindowEvent::Resized(new_width, new_height) => {
                    self.renderer
                        .resize_swapchain((new_width as u32, new_height as u32));
                }
                _ => {}
            },
            sdl3::event::Event::KeyDown {
                keycode: Some(key), ..
            } => match key {
                sdl3::keyboard::Keycode::Escape => {
                    std::process::exit(0);
                }
                _ => {}
            },
            _ => {}
        };

        self.input.handle_event(&event, true, true);
    }

    #[profiling::function]
    pub fn render(&mut self, event_pump: &sdl3::EventPump) {
        let now = std::time::Instant::now();
        let delta_time = (now - self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        let mut movement = Vec3::ZERO;
        if self.input.is_key_down(Keycode::W) {
            movement += self.camera.forward();
        }
        if self.input.is_key_down(Keycode::S) {
            movement -= self.camera.forward();
        }
        if self.input.is_key_down(Keycode::A) {
            movement -= self.camera.right();
        }
        if self.input.is_key_down(Keycode::D) {
            movement += self.camera.right();
        }
        if self.input.is_key_down(Keycode::Space) {
            movement += self.camera.up();
        }
        movement = movement.normalize_or(Vec3::ZERO);
        if self.input.is_key_down(Keycode::LCtrl) {
            movement /= 2.5;
        }
        if self.input.is_key_down(Keycode::LShift) {
            movement *= 2.5;
        }

        self.camera.position += movement * delta_time * 40.0;

        if self.input.is_mouse_button_down(MouseButton::Left) {
            self.yaw_pitch += (self.input.mouse_delta / 10.0) * vec2(-1.0, 1.3);
            self.yaw_pitch.y = self.yaw_pitch.y.clamp(-89.0, 89.0)
        }

        self.camera.rotation = Quat::from_rotation_z(self.yaw_pitch.x.to_radians())
            * Quat::from_rotation_y(self.yaw_pitch.y.to_radians());

        let resolution = self.renderer.surfaces.framebuffer_resolution();
        let proj = self
            .camera
            .projection_matrix(resolution.0 as f32 / resolution.1 as f32);
        // proj.z_axis.z = 2.6226E-06;

        let view = self.camera.view_matrix();

        self.renderer.frame_packet.write().reset();

        {
            let mut fp = self.renderer.frame_packet.write();
            for t in &self.static_render_objects {
                fp.push_static_render_object(t.clone());
            }
        }

        {
            self.renderer.begin_frame();

            {
                profiling::scope!("prepare");
                let gpu = &self.renderer.gpu;
                let context = gpu.context();

                context.clear_render_target_view(&gpu.acquire_rtv(), &[0.0, 0.0, 0.0, 1.0]);

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

                context.rasterizer_set_viewports(&[d3d11::Viewport::builder()
                    .width(self.renderer.surfaces.framebuffer_resolution().0 as f32)
                    .height(self.renderer.surfaces.framebuffer_resolution().1 as f32)
                    .build()]);

                context.output_merger_set_blend_state(None, None, 0xFFFFFFFF);
            }

            if !self.renderer.frame_packet.read().frame_nodes.is_empty() {
                let mut cmd = self.renderer.gpu.create_command_list();
                self.renderer.submit_world(
                    &mut cmd,
                    view,
                    proj,
                    self.start_time.elapsed().as_secs_f32(),
                    delta_time,
                );
                // let cmd = self.draw_world(delta_time);
                self.renderer.gpu.submit_command_list(cmd);
            } else {
                let gpu = &self.renderer.gpu;
                let context = gpu.context();
                context.output_merger_set_render_targets(&[Some(gpu.acquire_rtv())], None);
                context.output_merger_set_depth_stencil_state(None, 0);
                context.rasterizer_set_state(None);
                self.spinner.draw(&gpu);
            }

            {
                let mut debug_text = self.renderer.debug_text.lock();
                debug_text.add_string(
                    format!(
                        "Camera Position <{:.2},{:.2},{:.2}>",
                        self.camera.position.x, self.camera.position.y, self.camera.position.z
                    ),
                    IVec2::new(2, -3),
                    [0, 255, 255, 255],
                    DebugTextAlign::BottomLeft,
                );
                debug_text.add_string(
                    format!(
                        "Frame time {:.2} ms / {:.2} FPS",
                        delta_time * 1000.0,
                        1.0 / delta_time
                    ),
                    IVec2::new(2, -2),
                    [0, 255, 255, 255],
                    DebugTextAlign::BottomLeft,
                );
            }

            // if self.show_debug_text
            {
                let gpu = &self.renderer.gpu;
                let context = gpu.context();

                context.rasterizer_set_viewports(&[d3d11::Viewport::builder()
                    .width(gpu.swapchain_resolution().0 as f32)
                    .height(gpu.swapchain_resolution().1 as f32)
                    .build()]);
                context.output_merger_set_render_targets(&[Some(gpu.acquire_rtv())], None);
                context.output_merger_set_depth_stencil_state(None, 0);
                context.rasterizer_set_state(None);
                self.renderer.debug_text.lock().draw(&self.renderer.gpu);
            }
        }

        self.renderer.present_frame(true);

        self.input.update_keystates();
        profiling::finish_frame!();
    }
}
