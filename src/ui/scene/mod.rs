use std::{sync::Arc, time::Instant};

use deimos_data::tfx::RenderStage;
use deimos_render::{camera::Camera, gpu::command_list::CommandList, renderer::Renderer};
use egui::{RichText, Sense, Ui, UiBuilder, Vec2, containers::menu::MenuConfig, vec2};
use google_material_symbols::GoogleMaterialSymbols;
use hecs::World;

use crate::ui::scene::controller::CameraController;

pub mod controller;

pub struct Scene {
    renderer: Arc<Renderer>,

    // Camera/view
    camera: Camera,
    controller: CameraController,

    // World
    world: World,

    // Metrics
    frametimes: Vec<f32>,
    last_frame_time: Instant,

    // UI
    keep_settings_open: bool,
}

impl Scene {
    pub fn new(renderer: &Arc<Renderer>, camera: Camera) -> Self {
        Self {
            renderer: renderer.clone(),
            camera,
            controller: CameraController::new_first_person(),
            world: World::new(),
            frametimes: Vec::new(),
            last_frame_time: Instant::now(),
            keep_settings_open: false,
        }
    }

    pub fn with_controller(mut self, controller: CameraController) -> Self {
        self.controller = controller;
        self
    }

    pub fn set_world(&mut self, world: World) {
        self.world = world;
    }

    pub fn take_world(&mut self) -> hecs::World {
        std::mem::take(&mut self.world)
    }

    pub fn clear(&mut self) {
        self.world.clear();
    }
}

impl Scene {
    pub fn show(&mut self, ui: &mut egui::Ui, size: Vec2) {
        let now = Instant::now();
        let delta_time = (now - self.last_frame_time).as_secs_f32();
        self.frametimes.push(delta_time);
        while self.frametimes.len() > 60 {
            self.frametimes.remove(0);
        }

        self.last_frame_time = now;
        let delta_time_average = if !self.frametimes.is_empty() {
            let sum: f32 = self.frametimes.iter().sum();
            sum / self.frametimes.len() as f32
        } else {
            delta_time
        };

        egui::CentralPanel::default().show_inside(ui, |ui| {
            let panel_rect = ui.available_rect_before_wrap();

            let r = ui.allocate_response(size, Sense::CLICK | Sense::DRAG | Sense::HOVER);
            // let r = ui
            //     .image(SizedTexture {
            //         id: egui_d3d11.textures_mut().allocate_dx_temporary(
            //             self.surface_srv.clone(),
            //             None,
            //             false,
            //         ),
            //         size,
            //     })
            //     .interact(Sense::CLICK | Sense::DRAG | Sense::HOVER);

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
            ui.scope_builder(UiBuilder::new().max_rect(bar_rect), |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    self.show_toolbar(ui);
                })
            });

            let fps_rect = ui.painter_at(panel_rect).text(
                panel_rect.right_top() + Vec2::new(0.0, 3.0) + Vec2::splat(1.0),
                egui::Align2::RIGHT_TOP,
                format!("{} ", (1. / delta_time_average).round()),
                egui::FontId::monospace(16.0),
                egui::Color32::BLACK,
            );

            ui.painter_at(panel_rect).text(
                panel_rect.right_top() + Vec2::new(0.0, 3.0),
                egui::Align2::RIGHT_TOP,
                format!("{} ", (1. / delta_time_average).round()),
                egui::FontId::monospace(16.0),
                egui::Color32::GREEN,
            );

            ui.scope_builder(
                egui::UiBuilder::new().max_rect(panel_rect.shrink2(vec2(12.0, 4.0))),
                |ui| {
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                        if self.world.is_empty() {
                            ui.label(
                                RichText::new(format!(
                                    "{} Scene is empty",
                                    GoogleMaterialSymbols::Warning
                                ))
                                .size(16.0),
                            );
                        }

                        if self.renderer.asset_manager.count_loading() > 0 {
                            ui.label(
                                RichText::new(format!(
                                    "{} Loading assets... ({} in progress)",
                                    GoogleMaterialSymbols::HardDrive,
                                    self.renderer.asset_manager.count_loading()
                                ))
                                .size(16.0),
                            );
                        }
                    });
                },
            );

            ui.style_mut().spacing.tooltip_width = 4096.0;
            // Renderer::instance().profiler.set_enabled(false);
            // ui.interact(
            //     fps_rect,
            //     "frame_counter_profiler_tooltip".into(),
            //     Sense::hover(),
            // )
            // .on_hover_ui(|ui| {
            //     Renderer::instance().profiler.set_enabled(true);
            //     if let Some(profiler_results) = &self.profiler_results {
            //         ui.add(
            //             egui::Label::new(RichText::new(profiler_results.clone()).monospace())
            //                 .extend(),
            //         );
            //     } else {
            //         ui.weak("Profiler data not available yet.");
            //     }
            // });

            let size_pixels = size * ui.ctx().pixels_per_point();
            let resolution = (size_pixels.x as u32, size_pixels.y as u32);

            self.controller.update(&mut self.camera, ui, &r, delta_time);

            // if r.dragged_by(egui::PointerButton::Middle) {
            //     let delta_adjusted = r.drag_delta() / 4.0;
            //     self.sun_light_angle += delta_adjusted.x;
            //     self.sun_light_angle = self.sun_light_angle.rem_euclid(360.0);
            // }

            // self.render(delta_time, resolution);

            let cmd = &self.renderer.gpu.frame().command_list;
            let mut cmd_tfx =
                CommandList::from_native_command_list(&self.renderer, cmd.command_list.clone());

            {
                let ext = self.renderer.externs.get_mut();
                self.camera.aspect_ratio = resolution.0 as f32 / resolution.1 as f32;
                self.controller.update_rotation(&mut self.camera);
                self.camera.update();
                ext.view.world_to_camera = self.camera.world_to_camera;
                ext.view.camera_to_projective = self.camera.camera_to_projective;
                ext.view.derive_matrices(resolution);
            }
            self.renderer.globals.scopes.frame.bind(&mut cmd_tfx);
            self.renderer.globals.scopes.view.bind(&mut cmd_tfx);
            self.renderer.globals.scopes.chunk_model.bind(&mut cmd_tfx);
            for obj in self.renderer.objects.write().values_mut() {
                obj.renderer.extract(&self.renderer, &());
                obj.renderer
                    .submit(&mut cmd_tfx, RenderStage::GenerateGbuffer);
            }
        });
    }

    fn show_toolbar(&mut self, ui: &mut Ui) {
        ui.style_mut().spacing.item_spacing = vec2(8.0, 0.0);
        // egui::containers::menu::MenuButton::new(GoogleMaterialSymbols::Tune.to_string())
        //     .config(
        //         MenuConfig::new().close_behavior(if self.keep_settings_open {
        //             egui::PopupCloseBehavior::IgnoreClicks
        //         } else {
        //             egui::PopupCloseBehavior::CloseOnClickOutside
        //         }),
        //     )
        //     .ui(ui, |ui| {
        //         self.show_settings_ui(ui);
        //     })
        //     .0
        //     .on_hover_text("Scene Settings");

        if ui
            .selectable_label(
                false, // self.show_surface_viewer,
                GoogleMaterialSymbols::ImageSearch.to_string(),
            )
            .clicked()
        {
            // self.show_surface_viewer = !self.show_surface_viewer;
        }

        if ui
            .selectable_label(
                false, // self.show_channel_editor,
                GoogleMaterialSymbols::BarChart4Bars.to_string(),
            )
            .clicked()
        {
            // self.show_channel_editor = !self.show_channel_editor;
        }

        // self.render_mode.ui(ui);
        // self.view.subscribed_features.show_input(ui);
    }
}
