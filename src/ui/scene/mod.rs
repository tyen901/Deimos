use std::{sync::Arc, time::Instant};

use deimos_core::job::SCHEDULER;
use deimos_data::tfx::RenderStage;
use deimos_render::{
    camera::Camera,
    ecs::{populate_submit_nodes, s_extract_frame_packet},
    gpu::stream::ParallelCommandBlock,
    renderer::{Renderer, packet::FramePacket, scene::SceneRenderer},
    tfx::{externs::get_global_channel_name, view::ShadedView},
    util::range::RangeChunks,
    visibility::ViewVisibility,
};
use egui::{
    Color32, FontId, RichText, Sense, TextStyle, Ui, UiBuilder, Vec2, Widget, load::SizedTexture,
    vec2,
};
use google_material_symbols::GoogleMaterialSymbols;
use hecs::World;

use crate::ui::{scene::controller::CameraController, util::format_bytes};

pub mod controller;

pub struct Scene {
    renderer: Arc<Renderer>,
    scene: SceneRenderer,

    // Camera/view
    camera: Camera,
    controller: CameraController,

    // World
    pub world: World,

    // Metrics
    frametimes: Vec<f32>,
    last_frame_time: Instant,

    // UI
    keep_settings_open: bool,
    show_channel_editor: bool,
}

impl Scene {
    pub fn new(renderer: &Arc<Renderer>, camera: Camera) -> anyhow::Result<Self> {
        Ok(Self {
            scene: SceneRenderer::new(renderer.clone())?,
            renderer: renderer.clone(),
            camera,
            controller: CameraController::new_first_person(),
            world: World::new(),
            frametimes: Vec::new(),
            last_frame_time: Instant::now(),
            keep_settings_open: false,
            show_channel_editor: false,
        })
    }

    pub const fn with_controller(mut self, controller: CameraController) -> Self {
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
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        egui_d3d12: &mut egui_d3d12::D3D12Renderer,
        size: Vec2,
    ) {
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

        // if self.show_surface_viewer {
        //     egui::SidePanel::right("surface_viewer").show_inside(ui, |ui| {
        //         // self.show_texture_viewer(ui, egui_d3d11);
        //         self.show_surface_viewer(ui, egui_d3d11);
        //     });
        // }

        if self.show_channel_editor {
            egui::SidePanel::right("channel_editor").show_inside(ui, |ui| {
                self.show_channel_editor(ui);
            });
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            let panel_rect = ui.available_rect_before_wrap();

            // let r = ui.allocate_response(size, Sense::CLICK | Sense::DRAG | Sense::HOVER);
            let r = ui
                .image(SizedTexture {
                    id: egui_d3d12.textures_mut().allocate_dx_temporary(
                        self.scene.main_view.gbuffer.albedo.srv().cpu_handle(),
                        None,
                        false,
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

            let memory_stats = self.renderer.gpu.memory_stats();
            let mut vram_text = format_bytes(memory_stats.allocator_used as usize);
            let mut vram_color = Color32::GREEN;
            if !memory_stats.errors.is_empty() {
                vram_text = format!("{} {}", GoogleMaterialSymbols::Error, vram_text);
                vram_color = Color32::RED;
            } else if !memory_stats.warnings.is_empty() {
                vram_text = format!("{} {}", GoogleMaterialSymbols::Warning, vram_text);
                vram_color = Color32::YELLOW;
            }

            let vram_rect = ui.painter_at(panel_rect).text(
                panel_rect.right_top() + Vec2::new(-8.0, 23.0) + Vec2::splat(1.0),
                egui::Align2::RIGHT_TOP,
                &vram_text,
                egui::FontId::monospace(16.0),
                egui::Color32::BLACK,
            );

            ui.painter_at(panel_rect).text(
                panel_rect.right_top() + Vec2::new(-8.0, 23.0),
                egui::Align2::RIGHT_TOP,
                vram_text,
                egui::FontId::monospace(16.0),
                vram_color,
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
            ui.interact(
                fps_rect,
                "frame_counter_profiler_tooltip".into(),
                Sense::hover(),
            )
            .on_hover_ui(|ui| {
                let profiler_results = self.renderer.gpu.frame().profiler.get_results_string();
                ui.add(egui::Label::new(RichText::new(profiler_results).monospace()).extend());
            });
            ui.interact(
                vram_rect,
                "vram_report_profiler_tooltip".into(),
                Sense::hover(),
            )
            .on_hover_ui(|ui| {
                ui.style_mut().spacing.item_spacing = vec2(0.0, 0.0);
                ui.monospace(format!("{} allocations", memory_stats.num_allocations));
                ui.monospace(format!(
                    "Capacity:  {}",
                    format_bytes(memory_stats.allocator_capacity as usize)
                ));
                ui.monospace(format!(
                    "Descriptor Heap: {}/{} ({:.0}%)",
                    memory_stats.descriptor_heap_used,
                    memory_stats.descriptor_heap_capacity,
                    memory_stats.descriptor_heap_used as f32
                        / memory_stats.descriptor_heap_capacity as f32
                        * 100.0
                ));
                ui.separator();
                ui.monospace(format!(
                    "Upload Ring:  {}/{} ({})",
                    format_bytes(memory_stats.upload_ring_used),
                    format_bytes(memory_stats.upload_ring_capacity),
                    memory_stats.num_upload_ring_allocations
                ));
                ui.monospace(format!(
                    "Descriptor Ring:  {}/{}",
                    memory_stats.descriptor_ring_used, memory_stats.descriptor_ring_capacity
                ));

                if !memory_stats.errors.is_empty() {
                    ui.separator();
                    ui.colored_label(Color32::RED, "Errors:");
                    for error in memory_stats.errors.iter() {
                        ui.monospace(format!(" - {error:?}"));
                    }
                } else if !memory_stats.warnings.is_empty() {
                    ui.separator();
                    ui.colored_label(Color32::YELLOW, "Warnings:");
                    for warning in memory_stats.warnings.iter() {
                        ui.monospace(format!(" - {warning:?}"));
                    }
                }
            });

            let size_pixels = size * ui.ctx().pixels_per_point();
            let resolution = (size_pixels.x as u32, size_pixels.y as u32);
            if let Err(e) = self.scene.main_view.resize(&self.renderer.gpu, resolution) {
                error!("Failed to resize main view: {e:?}");
            }

            self.controller.update(&mut self.camera, ui, &r, delta_time);

            // if r.dragged_by(egui::PointerButton::Middle) {
            //     let delta_adjusted = r.drag_delta() / 4.0;
            //     self.sun_light_angle += delta_adjusted.x;
            //     self.sun_light_angle = self.sun_light_angle.rem_euclid(360.0);
            // }

            self.render(delta_time, resolution);
        });
    }

    #[profiling::function]
    fn render(&mut self, delta_time: f32, resolution: (u32, u32)) {
        let stream = &self.renderer.gpu.frame().stream;
        let mut cmd_guard = stream.acquire_cmd(&self.renderer.gpu);
        let cmd = &mut *cmd_guard;

        {
            let ext = self.renderer.externs.get_mut();
            self.camera.aspect_ratio = resolution.0 as f32 / resolution.1 as f32;
            self.controller.update_rotation(&mut self.camera);
            self.camera.update();
            ext.view.world_to_camera = self.camera.world_to_camera;
            ext.view.camera_to_projective = self.camera.camera_to_projective;
            ext.view.derive_matrices(resolution);
            self.scene.main_view.culling_frustum = self.camera.culling_frustum.clone();
        }

        self.renderer.globals.scopes.frame.bind(cmd);
        self.renderer.globals.scopes.view.bind(cmd);
        self.renderer.globals.scopes.chunk_model.bind(cmd);

        {
            let _event = cmd.event_scope_str("Scene");
            let _scope = self.renderer.gpu.profiler_scope(stream, "scene");
            self.scene
                .main_view
                .gbuffer
                .transition(cmd, d3d12::ResourceStates::RENDER_TARGET);
            self.scene.main_view.gbuffer.clear(cmd);
            self.scene.main_view.gbuffer.bind(cmd);

            // s_extract_render_objects(&self.world, &self.renderer);

            // for (_entity, render_object) in self.world.query::<&DynamicRenderObject>().iter() {
            //     self.renderer.objects.read()[render_object.handle]
            //         .renderer
            //         .submit(cmd, RenderStage::GenerateGbuffer);
            // }
            self.scene.frame_packet.reset();
            let vis = &ViewVisibility {
                culling_frustum: self.camera.culling_frustum.clone(),
                position: self.camera.position,
                world_to_projective: self.camera.world_to_projective,
            };

            {
                let _scope = self.renderer.gpu.profiler_scope(stream, "extract_frame");
                s_extract_frame_packet(&self.world, &mut self.scene, vis);
            }
            {
                let _scope = self
                    .renderer
                    .gpu
                    .profiler_scope(stream, "populate_submit_nodes");
                populate_submit_nodes(&mut self.scene, vis);
            }

            let _scope = self
                .renderer
                .gpu
                .profiler_scope(stream, "submit_gbuffer_generation");
            for view in &self.scene.frame_packet.views {
                let block = stream.begin_parallel(cmd);
                let context = TempSubmitContext {
                    renderer: self.renderer.clone(),
                    block: block.clone(),
                    frame_packet: &self.scene.frame_packet,
                };

                let range = 0..view
                    .submit_node_blocks
                    .block(RenderStage::GenerateGbuffer)
                    .len();

                let mut job_handles = vec![];
                for chunk in RangeChunks::new(range, 128) {
                    let ctx = context.clone();
                    let h = SCHEDULER
                        .job_builder("scene_submit_parallel")
                        .spawn(move || {
                            let ctx = ctx;
                            let mut cmd = ctx.block.cmd();
                            let render_objects = ctx.renderer.objects.read();

                            let frame_packet = unsafe { &*ctx.frame_packet };
                            let view = &frame_packet.views[0];
                            for submit_node in
                                &view.submit_node_blocks.block(RenderStage::GenerateGbuffer)[chunk]
                            {
                                let view_node = &view.view_nodes[submit_node.view_node];
                                let frame_node =
                                    &frame_packet.per_frame_nodes[view_node.frame_node];
                                let Some(render_object) = render_objects.get(frame_node.object)
                                else {
                                    error!(
                                        "Render object with handle {:?} not found",
                                        frame_node.object
                                    );
                                    continue;
                                };
                                render_object.renderer.submit(
                                    &mut cmd,
                                    RenderStage::GenerateGbuffer,
                                    view_node,
                                    submit_node.key,
                                );
                            }
                        });
                    job_handles.push(h);
                }

                let sync_job = SCHEDULER
                    .job_builder("scene_submit_parallel_sync")
                    .dependencies(job_handles)
                    .spawn(|| {});

                sync_job.wait();

                // submit_node_block_range(
                //     context.clone(),
                //     0..view
                //         .submit_node_blocks
                //         .block(RenderStage::GenerateGbuffer)
                //         .len(),
                // );
                stream.end_parallel(block);

                // self.renderer.gpu.frame().cmd_pool.apply(
                //     &cmd_tfx,
                //     std::slice::from_ref(self.renderer.gpu.frame().descriptors.heap()),
                // );
                // for submit_node in view.submit_node_blocks.block(RenderStage::GenerateGbuffer) {
                //     let view_node = &view.view_nodes[submit_node.view_node];
                //     let frame_node = &self.scene.frame_packet.per_frame_nodes[view_node.frame_node];
                //     let Some(render_object) = render_objects.get(frame_node.object) else {
                //         error!(
                //             "Render object with handle {:?} not found",
                //             frame_node.object
                //         );
                //         continue;
                //     };
                //     render_object.renderer.submit(
                //         cmd,
                //         RenderStage::GenerateGbuffer,
                //         view_node,
                //         submit_node.key,
                //     );
                // }
            }

            self.scene
                .main_view
                .gbuffer
                .transition(cmd, d3d12::ResourceStates::PIXEL_SHADER_RESOURCE);
        }
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
                self.show_channel_editor,
                GoogleMaterialSymbols::BarChart4Bars.to_string(),
            )
            .clicked()
        {
            self.show_channel_editor = !self.show_channel_editor;
        }

        // self.render_mode.ui(ui);
        // self.view.subscribed_features.show_input(ui);
    }

    fn show_channel_editor(&mut self, ui: &mut Ui) {
        ui.style_mut()
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(16.0));
        ui.style_mut()
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(16.0));

        // let automated_ids = s_get_all_global_channel_ids(&self.world);
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Global Channels");
            for (i, channel) in self.scene.global_channels.iter_mut().enumerate() {
                let Some(channel_id) = self.renderer.externs.global_ids.get(i) else {
                    continue;
                };
                // let is_automated = automated_ids.contains(channel_id);
                let is_automated = false;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if let Some(name) = get_global_channel_name(*channel_id) {
                        ui.label(format!("Channel #{i} {name} (0x{channel_id:08X})"));
                    } else {
                        ui.label(format!("Channel #{i} 0x{channel_id:08X}"));
                    }
                    if is_automated {
                        ui.weak("(automated)");
                    }
                });
                ui.add_enabled_ui(!is_automated, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().button_padding = vec2(4.0, 1.0);
                        ui.spacing_mut().interact_size = egui::vec2(100.0, 32.0);
                        egui::DragValue::new(&mut channel.x)
                            .fixed_decimals(4)
                            .speed(0.01)
                            .ui(ui);
                        egui::DragValue::new(&mut channel.y)
                            .fixed_decimals(4)
                            .speed(0.01)
                            .ui(ui);
                        egui::DragValue::new(&mut channel.z)
                            .fixed_decimals(4)
                            .speed(0.01)
                            .ui(ui);
                        egui::DragValue::new(&mut channel.w)
                            .fixed_decimals(4)
                            .speed(0.01)
                            .ui(ui);
                    });
                });
            }
        });
    }
}

#[derive(Clone)]
struct TempSubmitContext {
    renderer: Arc<Renderer>,
    block: Arc<ParallelCommandBlock>,
    frame_packet: *const FramePacket,
}

unsafe impl Send for TempSubmitContext {}

// const RANGE_GRANULARITY: usize = 128;
// fn submit_node_block_range(ctx: TempSubmitContext, range: Range<usize>) {
//     if range.len() > RANGE_GRANULARITY {
//         let middle = range.len() / 2;
//         let range1 = range.start..range.start + middle;
//         let range2 = range.start + middle..range.end;
//         let ctx2 = ctx.clone();
//         rayon::join(
//             || submit_node_block_range(ctx, range1),
//             || submit_node_block_range(ctx2, range2),
//         );
//     } else {
//         let render_objects = ctx.renderer.objects.read();
//         let cmd = ctx.renderer.gpu.frame().cmd_pool.acquire();
//         let mut cmd_tfx =
//             CommandList::from_native_command_list(&ctx.renderer, cmd.command_list.clone());

//         let frame_packet = unsafe { &*ctx.frame_packet };
//         let view = &frame_packet.views[0];
//         for submit_node in &view.submit_node_blocks.block(RenderStage::GenerateGbuffer)[range] {
//             let view_node = &view.view_nodes[submit_node.view_node];
//             let frame_node = &frame_packet.per_frame_nodes[view_node.frame_node];
//             let Some(render_object) = render_objects.get(frame_node.object) else {
//                 error!(
//                     "Render object with handle {:?} not found",
//                     frame_node.object
//                 );
//                 continue;
//             };
//             render_object.renderer.submit(
//                 cmd,
//                 RenderStage::GenerateGbuffer,
//                 view_node,
//                 submit_node.key,
//             );
//         }
//         ctx.renderer.gpu.frame().cmd_pool.release(cmd);
//     }
// }
