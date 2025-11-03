use std::collections::BTreeMap;

use deimos_data::{
    map::ComponentData,
    pattern::SPattern,
    tfx::{TfxFeatureRenderer, features::dynamic::SDynamicModel},
};
use deimos_render::{
    Renderer, camera::Camera, feature::rigid_model::DynamicModel, object::RenderObject,
    tfx::packet::CompactTransform,
};
use egui::{Color32, CornerRadius, FontId, Rect, Sense, TextStyle, Ui, Vec2, vec2};
use glam::{Mat4, Vec3, Vec4Swizzles};
use itertools::Itertools;
use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::{TagHash, package, package_manager};

use crate::{
    ui::{scene::Scene, util::spinner_image},
    world::{
        pattern::{spawn_pattern, spawn_pattern_from_header},
        transform::Transform,
    },
};

use super::TabResult;

struct EntityEntry {
    hash: TagHash,
    pattern: SPattern,
    world: Option<hecs::World>,
}

impl EntityEntry {
    fn has_model(&self) -> bool {
        self.pattern
            .components
            .iter()
            .any(|comp| comp.unk0.unk10.resource_type == 0x80808673)
    }
}

pub struct EntityListTab {
    packages: BTreeMap<u16, Vec<EntityEntry>>,
    show_entities_without_models: bool,

    current_package: u16,
    current_tag: TagHash,
    scene: Scene,
}

impl EntityListTab {
    pub fn new() -> Self {
        Self {
            packages: package_manager()
                .package_paths
                .keys()
                .filter_map(|id| {
                    let has_entities = package_manager().lookup.tag32_entries_by_pkg[id]
                        .iter()
                        .any(|e| e.reference == SPattern::ID.unwrap());

                    if has_entities {
                        Some((*id, vec![]))
                    } else {
                        None
                    }
                })
                .collect(),
            current_package: 0,
            show_entities_without_models: false,
            current_tag: TagHash::NONE,
            scene: Scene::new(Renderer::instance().clone(), Camera::default()).unwrap(),
        }
    }

    fn load_entries_for_pkg(&mut self, pkg_id: u16) {
        let Some(entries) = self.packages.get_mut(&pkg_id) else {
            return;
        };
        if !entries.is_empty() {
            return;
        }

        *entries = package_manager().lookup.tag32_entries_by_pkg[&pkg_id]
            .iter()
            .enumerate()
            .filter(|(_, e)| e.reference == SPattern::ID.unwrap())
            .filter_map(|(i, _)| {
                let hash = TagHash::new(pkg_id, i as u16);
                match package_manager().read_tag_struct::<SPattern>(hash) {
                    Ok(pattern) => {
                        let mut world = hecs::World::new();
                        if let Err(e) = spawn_pattern_from_header(&mut world, &pattern, None) {
                            error!("Failed to load pattern {hash}: {e}");
                        }
                        Some(EntityEntry {
                            hash,
                            pattern,
                            world: Some(world),
                        })
                    }
                    Err(err) => {
                        error!("Failed to read pattern tag {hash}: {err}",);
                        None
                    }
                }
            })
            .collect();
    }

    pub fn ui(&mut self, ui: &mut Ui, egui_d3d11: &mut egui_d3d11::D3D11Renderer) -> TabResult {
        self.load_entries_for_pkg(self.current_package);

        ui.separator();
        ui.style_mut()
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(16.0));
        ui.style_mut().spacing.button_padding = Vec2::new(8.0, 4.0);

        egui::SidePanel::left("entities_packages_list").show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let mut pkg_to_clear: Option<u16> = None;
                    for pkg_id in self.packages.keys() {
                        let path = &package_manager().package_paths[pkg_id];
                        if ui
                            .selectable_label(
                                *pkg_id == self.current_package,
                                format!("{:04x}: {}", pkg_id, path.name),
                            )
                            .clicked()
                        {
                            pkg_to_clear = Some(*pkg_id);
                            self.current_package = *pkg_id;
                        }
                    }

                    if let Some(pkg_id) = pkg_to_clear
                        && let Some(entries) = self.packages.get_mut(&pkg_id)
                    {
                        entries.clear();
                    }
                });
        });

        egui::SidePanel::right("entities_scene")
            .default_width(ui.ctx().screen_rect().width() * 0.3)
            .show_inside(ui, |ui| {
                self.scene.show(ui, ui.available_size(), egui_d3d11);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.show_entities_without_models,
                    "Show entities without models",
                );
                ui.separator();
                if let Some(entries) = self.packages.get(&self.current_package) {
                    let total = entries.len();
                    let with_models = entries.iter().filter(|e| e.has_model()).count();
                    ui.label(format!(
                        "Showing {}/{} entities",
                        if self.show_entities_without_models {
                            total
                        } else {
                            with_models
                        },
                        total
                    ));
                }
            });
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .drag_to_scroll(false)
                .show(ui, |ui| {
                    let Some(entries) = self.packages.get(&self.current_package) else {
                        ui.label("No package selected");
                        return;
                    };
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(16.0, 16.0);
                        for entity in entries {
                            if !self.show_entities_without_models && !entity.has_model() {
                                continue;
                            }
                            const TAG_BOX_HEIGHT: f32 = 30.0;
                            let (card_rect, card_response) = ui.allocate_exact_size(
                                vec2(256.0, 256.0 + TAG_BOX_HEIGHT),
                                Sense::click(),
                            );

                            let card_image_rect =
                                card_rect.with_max_y(card_rect.max.y - TAG_BOX_HEIGHT);
                            let card_painter = ui.painter_at(card_rect);
                            card_painter.rect_filled(
                                card_rect,
                                8.0,
                                ui.visuals().widgets.inactive.bg_fill,
                            );
                            card_painter.rect_filled(
                                card_rect.with_min_y(card_rect.max.y - TAG_BOX_HEIGHT),
                                CornerRadius {
                                    se: 8,
                                    sw: 8,
                                    ..Default::default()
                                },
                                Color32::BLACK,
                            );
                            card_painter.text(
                                card_rect.left_bottom() + vec2(8.0, -3.0),
                                egui::Align2::LEFT_BOTTOM,
                                entity.hash.to_string(),
                                FontId::proportional(16.0),
                                ui.visuals().text_color(),
                            );
                            egui::Image::new(spinner_image().clone()).paint_at(
                                ui,
                                Rect::from_center_size(card_image_rect.center(), vec2(64.0, 64.0)),
                            );

                            if card_response.hovered() || entity.hash == self.current_tag {
                                let opacity = if entity.hash == self.current_tag {
                                    1.0
                                } else {
                                    0.5
                                };
                                card_painter.rect_stroke(
                                    card_rect.shrink(2.0),
                                    8.0,
                                    (
                                        2.0,
                                        ui.visuals()
                                            .widgets
                                            .hovered
                                            .fg_stroke
                                            .color
                                            .gamma_multiply(opacity),
                                    ),
                                    egui::StrokeKind::Outside,
                                );
                            }

                            if card_response.clicked() {
                                self.current_tag = entity.hash;

                                self.scene.clear();
                                match spawn_pattern(&mut self.scene.world, entity.hash, None) {
                                    Ok(entity) => {
                                        self.scene.world.insert_one(entity, Transform::default());
                                        self.scene.focus_on(Vec3::ZERO);
                                        // self.scene.focus_fit(model.model.bounding_sphere());
                                        // self.scene.focus_on(model.model.model_offset.xyz());
                                        // self.scene.add_dynamic_object(
                                        //     RenderObject::new(
                                        //         TfxFeatureRenderer::RigidObject,
                                        //         model,
                                        //         Box::new(CompactTransform::IDENTITY),
                                        //     ),
                                        //     Mat4::IDENTITY,
                                        // );
                                    }
                                    Err(err) => {
                                        error!("Failed to load model: {err}");
                                    }
                                }
                            }
                        }
                    });
                });
        });

        TabResult::Continue
    }
}
