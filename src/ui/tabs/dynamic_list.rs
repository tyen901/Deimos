use std::collections::BTreeMap;

use deimos_data::tfx::{features::dynamic::SDynamicModel, TfxFeatureRenderer};
use deimos_render::{
    camera::Camera, feature::rigid_model::DynamicModel, object::RenderObject,
    tfx::packet::CompactTransform, Renderer,
};
use egui::{FontId, TextStyle, Ui, Vec2};
use glam::{Mat4, Vec3, Vec4Swizzles};
use itertools::Itertools;
use tiger_parse::TigerReadable;
use tiger_pkg::{package_manager, TagHash};

use crate::ui::scene::Scene;

use super::TabResult;

pub struct DynamicListTab {
    packages: BTreeMap<u16, Vec<TagHash>>,
    tag_lookup_input: String,

    current_package: u16,
    current_tag: TagHash,
    scene: Scene,
}

impl DynamicListTab {
    pub fn new() -> Self {
        Self {
            packages: package_manager()
                .package_paths
                .keys()
                .filter_map(|id| {
                    let tags = package_manager().lookup.tag32_entries_by_pkg[id]
                        .iter()
                        .enumerate()
                        .filter(|(_, e)| e.reference == SDynamicModel::ID.unwrap())
                        .map(|(i, _)| TagHash::new(*id, i as u16))
                        .collect_vec();

                    if tags.is_empty() {
                        None
                    } else {
                        Some((*id, tags))
                    }
                })
                .collect(),
            current_package: 0,
            tag_lookup_input: String::new(),
            current_tag: TagHash::NONE,
            scene: Scene::new(
                Renderer::instance().clone(),
                Camera {
                    near: 0.01,
                    ..Default::default()
                },
            )
            .unwrap(),
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, egui_d3d11: &mut egui_d3d11::D3D11Renderer) -> TabResult {
        // egui::TextEdit::singleline(&mut self.tag_lookup_input)
        //     .hint_text(RichText::new("Quick Tag Lookup").weak().italics())
        //     .ui(ui);

        ui.separator();
        ui.style_mut()
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(16.0));
        ui.style_mut().spacing.button_padding = Vec2::new(8.0, 4.0);

        egui::SidePanel::left("dynamics_packages_list").show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for pkg_id in self.packages.keys() {
                        let path = &package_manager().package_paths[pkg_id];
                        if ui
                            .selectable_label(
                                *pkg_id == self.current_package,
                                format!("{:04x}: {}", pkg_id, path.name),
                            )
                            .clicked()
                        {
                            self.current_package = *pkg_id;
                        }
                    }
                });
        });
        egui::SidePanel::left("dynamics_entry_list").show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let Some(entries) = self.packages.get(&self.current_package) else {
                        ui.label("No package selected");
                        return;
                    };
                    for tag in entries {
                        if ui
                            .selectable_label(self.current_tag == *tag, format!("{tag}"))
                            .clicked()
                        {
                            self.current_tag = *tag;

                            self.scene.clear();
                            match DynamicModel::load(*tag, vec![], vec![]) {
                                Ok(model) => {
                                    self.scene.focus_on(model.model.model_offset.xyz());
                                    self.scene.add_dynamic_object(
                                        RenderObject::new(
                                            TfxFeatureRenderer::RigidObject,
                                            model,
                                            Box::new(CompactTransform::IDENTITY),
                                        ),
                                        Mat4::IDENTITY,
                                    );
                                }
                                Err(err) => {
                                    error!("Failed to load model: {err}");
                                }
                            }
                        }
                    }
                });
        });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.scene.show(ui, ui.available_size(), egui_d3d11);
        });

        TabResult::Continue
    }
}
