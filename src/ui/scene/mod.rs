pub mod controller;

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

use bitflags::Flags;
use d3d11::{dxgi, ShaderResourceView, Texture2D, Texture2dDesc};
use deimos_data::tfx::FeatureRendererSubscription;
use deimos_render::{
    camera::Camera,
    gpu::command_list::CommandList,
    object::{RenderObject, RenderObjectHandle},
    renderer::submit::DebugPipeline,
    tfx::{packet::CompactTransform, view::View},
    Gpu, Renderer,
};
use egui::{load::SizedTexture, vec2, FontId, RichText, Sense, TextStyle, Ui, UiBuilder, Vec2};
use glam::{Mat4, Vec3};
use google_material_symbols::GoogleMaterialSymbols;

use crate::ui::{
    scene::controller::CameraController,
    util::{ExternalDataWidgetExt, UiExt},
};

pub struct Scene {
    renderer: Arc<Renderer>,
    camera: Camera,
    view: View,
    last_frame_time: Instant,
    sun_light_angle: f32,
    render_mode: RenderMode,

    controller: CameraController,

    static_render_objects: Vec<RenderObjectHandle>,
    dynamic_render_objects: Vec<(RenderObjectHandle, Mat4)>,

    surface: d3d11::Texture2D,
    surface_srv: d3d11::ShaderResourceView,

    profiler_results: Option<String>,
}

impl Scene {
    pub fn new(renderer: Arc<Renderer>, camera: Camera) -> anyhow::Result<Self> {
        let (surface, surface_srv) = Self::create_surface(&renderer.gpu, (128, 128))?;

        Ok(Self {
            view: View::new(&renderer.gpu, (128, 128))?,
            renderer,
            camera,
            sun_light_angle: -120f32,
            render_mode: RenderMode::Shaded,
            controller: CameraController::new_orbit(Vec3::ZERO, 3.5),
            static_render_objects: Vec::new(),
            dynamic_render_objects: Vec::new(),
            surface,
            surface_srv,
            last_frame_time: Instant::now(),
            profiler_results: None,
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
                .format(dxgi::Format::R8g8b8a8Unorm)
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

    pub fn add_dynamic_object(&mut self, object: RenderObject, transform: Mat4) {
        self.dynamic_render_objects
            .push((self.renderer.add_object(object), transform));
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

        let mut bar_rect = r.rect;
        bar_rect.set_height(32.0);
        ui.painter().rect_filled(
            bar_rect,
            0.0,
            egui::Color32::from_black_alpha(if ui.rect_contains_pointer(bar_rect) {
                160
            } else {
                64
            }),
        );
        ui.allocate_new_ui(UiBuilder::new().max_rect(bar_rect), |ui| {
            egui::menu::bar(ui, |ui| {
                self.show_toolbar(ui);
            })
        });

        let now = Instant::now();
        let delta_time = (now - self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        let fps_rect = ui.painter_at(r.rect).text(
            r.rect.right_top() + Vec2::new(0.0, 3.0) + Vec2::splat(1.0),
            egui::Align2::RIGHT_TOP,
            format!("{} ", (1. / delta_time).round()),
            egui::FontId::monospace(16.0),
            egui::Color32::BLACK,
        );

        ui.painter_at(r.rect).text(
            r.rect.right_top() + Vec2::new(0.0, 3.0),
            egui::Align2::RIGHT_TOP,
            format!("{} ", (1. / delta_time).round()),
            egui::FontId::monospace(16.0),
            egui::Color32::GREEN,
        );

        ui.style_mut().spacing.tooltip_width = 4096.0;
        ui.interact(
            fps_rect,
            "frame_counter_profiler_tooltip".into(),
            Sense::hover(),
        )
        .on_hover_ui(|ui| {
            if let Some(profiler_results) = &self.profiler_results {
                ui.add(
                    egui::Label::new(RichText::new(profiler_results.clone()).monospace()).extend(),
                );
            } else {
                ui.weak("Profiler data not available yet.");
            }
        });

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

        if r.dragged_by(egui::PointerButton::Secondary) {
            let delta_adjusted = r.drag_delta() / 4.0;
            self.sun_light_angle += delta_adjusted.x;
            self.sun_light_angle = self.sun_light_angle.rem_euclid(360.0);
        }

        self.render(delta_time);
    }

    fn show_toolbar(&mut self, ui: &mut Ui) {
        ui.style_mut().spacing.item_spacing = vec2(8.0, 0.0);
        ui.label("");
        self.render_mode.ui(ui);
        self.view.subscribed_features.show_input(ui);
    }

    fn render(&mut self, delta_time: f32) {
        let gpu = &self.renderer.gpu;
        let mut cmd = CommandList::from_device_context(gpu, gpu.context().clone());
        let _gpuspan = self.renderer.profiler.scope(&cmd, "Scene::render (total)");
        self.renderer.frame_packet.write().reset();

        let sun_light_direction = Vec3::new(
            self.sun_light_angle.to_radians().cos(),
            self.sun_light_angle.to_radians().sin(),
            -0.7,
        )
        .normalize();

        self.renderer
            .externs
            .get_mut()
            .set_global_channel_by_name("sun_light_direction", sun_light_direction.extend(0.0));

        {
            let mut fp = self.renderer.frame_packet.write();
            for t in &self.static_render_objects {
                fp.push_static_render_object(*t);
            }
            for (t, transform) in &self.dynamic_render_objects {
                fp.push_dynamic_render_object(*t, CompactTransform::from_mat4(*transform));
            }
        }

        {
            profiling::scope!("prepare");
            let _gpuspan = self.renderer.profiler.scope(&cmd, "prepare");

            cmd.clear_render_target_view(&gpu.acquire_rtv(), &[0.0, 0.0, 0.0, 1.0]);

            {
                profiling::scope!("visibility");
                let _gpuspan = self.renderer.profiler.scope(&cmd, "visibility");
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
                            if !self
                                .view
                                .subscribed_features
                                .is_subscribed(render_object.feature_type)
                            {
                                return false;
                            }
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

        self.renderer
            .submit_world(&mut cmd, &self.view, delta_time, self.render_mode.into());
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

        drop(_gpuspan);
        self.renderer.profiler.end_frame();

        static FRAME_COUNT: AtomicUsize = AtomicUsize::new(0);
        if FRAME_COUNT
            .fetch_add(1, Ordering::Relaxed)
            .is_multiple_of(10)
        {
            self.profiler_results = Some(self.renderer.profiler.get_results_string());
        }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderMode {
    Shaded,
    // Matcap,

    // Material:
    Albedo,
    Smoothness,
    Metalness,
    AmbientOcclusion,
    Emission,
    Transmission,
    IridescenceId,

    // Geometry:
    DepthEdges,
    WorldNormal,
}

impl RenderMode {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.style_mut()
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(16.0));

        egui::ComboBox::from_id_salt("Render Mode")
            .height(400.0)
            .selected_text(format!("{} {:?}", GoogleMaterialSymbols::EvShadow, self))
            .show_ui(ui, |ui| {
                ui.style_mut()
                    .text_styles
                    .insert(TextStyle::Button, FontId::proportional(16.0));
                ui.style_mut().spacing.button_padding = Vec2::new(8.0, 2.0);
                ui.style_mut().spacing.item_spacing = Vec2::ZERO;

                ui.selectable_value(self, RenderMode::Shaded, "Shaded");
                // ui.selectable_value(self, RenderMode::Matcap, "Matcap");

                ui.section_separator("Material:");
                ui.selectable_value(self, RenderMode::Albedo, "Albedo");
                // ui.selectable_value(self, RenderMode::Normals, "Normals");
                ui.selectable_value(self, RenderMode::Smoothness, "Smoothness");
                ui.selectable_value(self, RenderMode::Metalness, "Metalness");
                ui.selectable_value(self, RenderMode::AmbientOcclusion, "Ambient Occlusion");
                ui.selectable_value(self, RenderMode::Emission, "Emission");
                ui.selectable_value(self, RenderMode::Transmission, "Transmission");
                ui.selectable_value(self, RenderMode::IridescenceId, "Iridescence ID");

                ui.section_separator("Geometry:");
                ui.selectable_value(self, RenderMode::DepthEdges, "Depth Edges");
                ui.selectable_value(self, RenderMode::WorldNormal, "World Normal");
            });
    }
}

impl From<RenderMode> for Option<DebugPipeline> {
    fn from(val: RenderMode) -> Self {
        match val {
            RenderMode::Shaded => None,
            // RenderMode::Matcap => Some(DebugPipeline::Matcap),
            RenderMode::Albedo => Some(DebugPipeline::Albedo),
            RenderMode::Smoothness => Some(DebugPipeline::Smoothness),
            RenderMode::Metalness => Some(DebugPipeline::Metalness),
            RenderMode::AmbientOcclusion => Some(DebugPipeline::AmbientOcclusion),
            RenderMode::Emission => Some(DebugPipeline::Emission),
            RenderMode::Transmission => Some(DebugPipeline::Transmission),
            RenderMode::IridescenceId => Some(DebugPipeline::Overcoat),
            RenderMode::DepthEdges => Some(DebugPipeline::DepthEdges),
            RenderMode::WorldNormal => Some(DebugPipeline::WorldNormal),
        }
    }
}

impl ExternalDataWidgetExt for FeatureRendererSubscription {
    fn show_input(&mut self, ui: &mut Ui) -> egui::Response {
        ui.style_mut()
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(16.0));

        egui::ComboBox::from_id_salt("Feature Renderers")
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .height(400.0)
            .selected_text(format!(
                "{} Enabled Features",
                GoogleMaterialSymbols::CheckBox
            ))
            .show_ui(ui, |ui| {
                ui.style_mut()
                    .text_styles
                    .insert(TextStyle::Button, FontId::proportional(16.0));
                ui.style_mut().spacing.button_padding = Vec2::new(8.0, 2.0);
                ui.style_mut().spacing.item_spacing = Vec2::ZERO;

                let ctrl = ui.input(|i| i.modifiers.ctrl);
                let alt = ui.input(|i| i.modifiers.alt);
                macro_rules! feature {
                    ($flag:expr, $name:literal) => {
                        if ui.selectable_label(self.contains($flag), $name).clicked() {
                            if ctrl {
                                self.clear();
                                self.insert($flag);
                            } else if alt {
                                *self = FeatureRendererSubscription::all();
                                self.remove($flag);
                            } else if self.contains($flag) {
                                self.remove($flag);
                            } else {
                                self.insert($flag);
                            }
                        }
                    };
                }

                feature!(
                    FeatureRendererSubscription::STATIC_OBJECTS,
                    "Static Objects"
                );
                feature!(
                    FeatureRendererSubscription::TERRAIN_PATCH,
                    "Terrain Patches"
                );
                feature!(FeatureRendererSubscription::RIGID_OBJECT, "Rigid Objects");
                feature!(
                    FeatureRendererSubscription::SKY_TRANSPARENT,
                    "Sky Transparents"
                );
                feature!(FeatureRendererSubscription::SPEEDTREE_TREES, "Decorators");
                feature!(
                    FeatureRendererSubscription::DYNAMIC_DECALS,
                    "Dynamic Decals"
                );
                feature!(FeatureRendererSubscription::WATER, "Water");
                feature!(FeatureRendererSubscription::LENS_FLARES, "Lens Flares");
                feature!(FeatureRendererSubscription::PARTICLES, "Particles");
                ui.section_separator("Lighting");
                feature!(FeatureRendererSubscription::CUBEMAPS, "Cubemaps");
                feature!(
                    FeatureRendererSubscription::CHUNKED_LIGHTS,
                    "Chunked Lights"
                );
                feature!(
                    FeatureRendererSubscription::DEFERRED_LIGHTS,
                    "Deferred Lights"
                );
            })
            .response
    }
}
