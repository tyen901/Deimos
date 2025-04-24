use std::{f32, rc::Rc, sync::Arc, time::Instant};

use deimos_data::{map::SBubbleParent, tfx::TfxFeatureRenderer};
use deimos_render::{
    camera::Camera,
    gpu::{command_list::CommandList, debug_text::DebugTextAlign, spinner::FullscreenSpinner},
    object::{RenderObject, RenderObjectHandle},
    Gpu, Renderer,
};
use glam::{vec2, vec3, IVec2, Quat, Vec2, Vec3};
use sdl3::{keyboard::Keycode, video::Window};
use tiger_parse::TigerReadable;
use tiger_pkg::package_manager;

use crate::{
    cli::AppArgs,
    input::{MouseButton, MouseKeyboardState},
    map::load_static_map,
};

pub struct App {
    pub sdl: Rc<sdl3::Sdl>,
    pub window: Rc<Window>,
    pub gpu: Arc<Gpu>,
    pub renderer: Arc<Renderer>,

    pub egui_d3d11: egui_d3d11::D3D11Renderer,
    pub egui_sdl3: egui_sdl3_platform::Platform,

    input: MouseKeyboardState,
    spinner: FullscreenSpinner,
    last_frame_time: Instant,
    start_time: Instant,
    camera: Camera,
    frametime_history: Vec<f32>,

    // map: StaticMapTemp,
    static_render_objects: Vec<RenderObjectHandle>,

    yaw_pitch: Vec2,
}

impl App {
    pub fn new(sdl: Rc<sdl3::Sdl>, window: Rc<Window>, args: AppArgs) -> anyhow::Result<Self> {
        let gpu = Arc::new(Gpu::create(&window)?);
        let renderer = Arc::new(Renderer::new(gpu.clone(), window.size())?);
        Renderer::set_instance(renderer.clone());

        let Some(map_hash) = args.map else {
            println!("No map specified. Available maps:");
            for (t, _) in package_manager().get_all_by_reference(SBubbleParent::ID.unwrap()) {
                println!(
                    " - {t} ({})",
                    package_manager().package_paths[&t.pkg_id()].filename
                );
            }

            return Err(anyhow::anyhow!("No map specified. Use `-m MAP_HASH`"));
        };

        // let map_marsh = TagHash(0x80A8C43F);
        // let map_perimeter = TagHash(0x80A75EAC);
        // std::random::random::<usize>() % 2

        let map = load_static_map(map_hash)?;

        let mut static_render_objects = Vec::new();
        for t in map.terrain {
            static_render_objects.push(renderer.add_object(RenderObject::new(
                TfxFeatureRenderer::TerrainPatch,
                Box::new(t),
                Box::new(()),
            )));
        }
        for s in map.models {
            static_render_objects.push(renderer.add_object(RenderObject::new(
                TfxFeatureRenderer::StaticObjects,
                Box::new(s),
                Box::new(()),
            )));
        }

        let camera = Camera {
            position: vec3(0.0, 0.0, 100.0),
            ..Default::default()
        };

        Ok(Self {
            input: MouseKeyboardState::new(sdl.clone(), window.clone()),
            spinner: FullscreenSpinner::create(&renderer.gpu)?,
            egui_d3d11: egui_d3d11::D3D11Renderer::new(&gpu)?,
            egui_sdl3: egui_sdl3_platform::Platform::new(window.size())?,
            renderer,
            sdl,
            window,
            gpu,

            last_frame_time: Instant::now(),
            start_time: Instant::now(),
            camera,
            frametime_history: Vec::new(),
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
                    self.egui_d3d11
                        .resize_buffers(&self.renderer.gpu, || {
                            self.renderer
                                .resize_swapchain((new_width as u32, new_height as u32));
                            Ok(())
                        })
                        .ok();
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
        self.egui_sdl3
            .handle_event(&event, &self.sdl, &self.sdl.video().unwrap());
    }

    #[profiling::function]
    pub fn render(&mut self, _event_pump: &sdl3::EventPump) {
        let frame_start = std::time::Instant::now();
        let refresh_rate = if !self.window.has_input_focus() && !self.window.has_mouse_focus() {
            10.0
        } else {
            self.window
                .get_display()
                .and_then(|d| d.get_mode())
                .map(|m| m.refresh_rate)
                .unwrap_or(60.0)
        };
        let frame_end =
            frame_start + std::time::Duration::from_millis((1000.0 / refresh_rate) as u64);

        let delta_time = (frame_start - self.last_frame_time).as_secs_f32();
        self.last_frame_time = frame_start;

        self.frametime_history.push(delta_time);
        if self.frametime_history.len() > 100 {
            self.frametime_history.remove(0);
        }

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
        if self.input.is_key_down(Keycode::Q) {
            movement -= self.camera.up();
        }
        if self.input.is_key_down(Keycode::E) {
            movement += self.camera.up();
        }
        movement = movement.normalize_or(Vec3::ZERO);
        if self.input.is_key_down(Keycode::LCtrl) {
            movement /= 5.0;
        }
        if self.input.is_key_down(Keycode::LShift) {
            movement *= 2.0;
        }
        if self.input.is_key_down(Keycode::Space) {
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
        self.camera.aspect_ratio = resolution.0 as f32 / resolution.1 as f32;
        let proj = self.camera.projection_matrix(self.camera.aspect_ratio);
        // proj.z_axis.z = 2.6226E-06;

        let view = self.camera.view_matrix();
        self.camera.update();
        // Renderer::instance()
        //     .immediate
        //     .lock()
        //     .frustum(&frustum, 0x00ffff);

        self.renderer.frame_packet.write().reset();

        {
            let mut fp = self.renderer.frame_packet.write();
            for t in &self.static_render_objects {
                fp.push_static_render_object(*t);
            }
        }

        {
            self.renderer.begin_frame();

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

                cmd.rasterizer_set_viewports(&[d3d11::Viewport::builder()
                    .width(self.renderer.surfaces.framebuffer_resolution().0 as f32)
                    .height(self.renderer.surfaces.framebuffer_resolution().1 as f32)
                    .build()]);

                cmd.output_merger_set_blend_state(None, None, 0xFFFFFFFF);
            }

            if !self.renderer.frame_packet.read().frame_nodes.is_empty() {
                self.renderer.submit_world(
                    &mut cmd,
                    view,
                    proj,
                    self.start_time.elapsed().as_secs_f32(),
                    delta_time,
                );
                // let cmd = self.draw_world(delta_time);
                // self.renderer.gpu.submit_command_list(cmd);
            } else {
                let gpu = &self.renderer.gpu;
                let context = gpu.context();
                context.output_merger_set_render_targets(&[Some(gpu.acquire_rtv())], None);
                context.output_merger_set_depth_stencil_state(None, 0);
                context.rasterizer_set_state(None);
                self.spinner.draw(gpu);
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
                let average_frame_time = self.frametime_history.iter().sum::<f32>()
                    / self.frametime_history.len() as f32;
                debug_text.add_string(
                    format!(
                        "Average frame time {:.2} ms / {:.2} FPS",
                        average_frame_time * 1000.0,
                        1.0 / average_frame_time
                    ),
                    IVec2::new(2, -1),
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

            let mut ctx = self
                .egui_sdl3
                .begin_frame(self.window.size(), self.window.display_scale());
            egui::Window::new("Demo Window").show(&ctx, |ui| {
                ui.label("Hello, World!");
            });
            let mut output = self
                .egui_sdl3
                .end_frame(&mut self.sdl.video().unwrap())
                .unwrap();
            self.egui_d3d11.paint(&mut cmd, output, &mut ctx);
        }

        let vsync = false;
        self.renderer.present_frame(vsync);
        if !vsync {
            spin_sleep::sleep_until(frame_end);
        }

        self.input.update_keystates();
        profiling::finish_frame!();
    }
}
