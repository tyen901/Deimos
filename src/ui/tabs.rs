use std::fmt::Display;

use deimos_data::{map::SBubbleParent, tfx::TfxFeatureRenderer};
use deimos_render::{camera::Camera, object::RenderObject, Renderer};
use egui::{vec2, Color32, Margin, Rect};
use egui_dock::{DockState, NodeIndex, SurfaceIndex, TabIndex};
use google_material_symbols::GoogleMaterialSymbols;
use tiger_parse::TigerReadable;
use tiger_pkg::{package_manager, TagHash};

use crate::{
    map::{load_static_map, StaticMapTemp},
    task::Task,
};

use super::{
    scene3d::Scene,
    util::{spinner_image, UiExt},
};

pub enum Tab {
    Home,
    Settings,
    DynamicList,
    MapList(Vec<TagHash>),
    Map {
        load_task: Task<StaticMapTemp>,
        tag: TagHash,
        scene: Box<Scene>,
    },
}

impl Tab {
    pub fn is_fixed(&self) -> bool {
        matches!(self, Tab::Home | Tab::Settings)
    }

    /// Returns an arbitrary key that's unique for the corresponding tab type. Tabs with only 1 instance return 0
    pub fn key(&self) -> u64 {
        match self {
            Tab::Home => 0,
            Tab::Settings => 0,
            Tab::DynamicList => 0,
            Tab::MapList(_) => 0,
            Tab::Map { tag, .. } => tag.0 as u64,
        }
    }
}

impl Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Tab::Settings => GoogleMaterialSymbols::Settings.to_string(),
            Tab::Home => format!("{} HOME", GoogleMaterialSymbols::Home),
            Tab::DynamicList => format!("{} DYNAMICS", GoogleMaterialSymbols::DeployedCode),
            Tab::MapList(_) => format!("{} MAPS", GoogleMaterialSymbols::Map),
            Tab::Map { tag, .. } => format!("Map {tag}"),
        };

        f.write_str(&s)
    }
}

pub struct TabViewer<'a> {
    pub added_nodes: &'a mut Vec<Tab>,
    pub egui_d3d11: &'a mut egui_d3d11::D3D11Renderer,
}

impl<'a> egui_dock::TabViewer for TabViewer<'a> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.to_string().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        egui::Frame::new()
            .outer_margin(if tab.is_fixed() {
                Margin::symmetric(127, 64)
            } else {
                Margin::ZERO
            })
            .show(ui, |ui| {
                ui.with_layout(
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| match tab {
                        Tab::Home => {
                            ui.add_space(32.0);
                            ui.columns(2, |uis| {
                                uis[0].heading("3D");
                                uis[0].add_space(4.0);
                                if uis[0]
                                    .d_button(format!(
                                        "{} DYNAMICS",
                                        GoogleMaterialSymbols::DeployedCode
                                    ))
                                    .clicked()
                                {
                                    self.added_nodes.push(Tab::DynamicList);
                                }
                                if uis[0]
                                    .d_button(format!("{} MAPS", GoogleMaterialSymbols::Map))
                                    .clicked()
                                {
                                    self.added_nodes.push(Tab::MapList(
                                        package_manager()
                                            .get_all_by_reference(SBubbleParent::ID.unwrap())
                                            .into_iter()
                                            .map(|(t, _)| t)
                                            .collect(),
                                    ));
                                }
                                uis[0].disable();
                                let _ = uis[0].d_button(format!(
                                    "{} STATICS",
                                    GoogleMaterialSymbols::Landscape
                                ));

                                uis[1].heading("2D");
                                uis[1].add_space(4.0);
                                uis[1].disable();
                                let _ = uis[1]
                                    .d_button(format!("{} TEXTURES", GoogleMaterialSymbols::Image));
                                let _ = uis[1].d_button(format!(
                                    "{} UI",
                                    GoogleMaterialSymbols::DesktopWindows
                                ));
                            });
                        }
                        Tab::Settings => {
                            ui.weak("No settings are available");
                        }
                        Tab::DynamicList => {
                            // ui.weak("Wompy");
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(ui.cursor().min, ui.available_size()),
                                0,
                                Color32::RED,
                            );
                        }
                        Tab::MapList(map_tags) => {
                            egui::Frame::new()
                                .outer_margin(Margin::symmetric(64, 64))
                                .show(ui, |ui| {
                                    for tag in map_tags {
                                        let tag = *tag;
                                        let path = &package_manager().package_paths[&tag.pkg_id()];
                                        if ui.d_button(format!("{} - {}", path.name, tag)).clicked()
                                        {
                                            self.added_nodes.push(Tab::Map {
                                                load_task: Task::new(move || {
                                                    load_static_map(tag).unwrap()
                                                }),
                                                tag,
                                                scene: Box::new(
                                                    Scene::new(
                                                        Renderer::instance().clone(),
                                                        Camera::default(),
                                                    )
                                                    .unwrap(),
                                                ),
                                            });
                                        }
                                    }
                                });
                        }
                        Tab::Map {
                            load_task, scene, ..
                        } => {
                            if let Some(map) = load_task.get() {
                                match map {
                                    Ok(map) => {
                                        for t in map.terrain {
                                            scene.add_static_object(RenderObject::new(
                                                TfxFeatureRenderer::TerrainPatch,
                                                Box::new(t),
                                                Box::new(()),
                                            ));
                                        }
                                        for s in map.models {
                                            scene.add_static_object(RenderObject::new(
                                                TfxFeatureRenderer::StaticObjects,
                                                Box::new(s),
                                                Box::new(()),
                                            ));
                                        }
                                    }
                                    Err(_e) => {
                                        error!("Failed to load map: unknown error");
                                    }
                                }
                            }

                            if load_task.is_pending() {
                                let (_, rect) = ui.allocate_space(ui.available_size());
                                ui.painter()
                                    .rect_filled(rect, 0, Color32::from_rgb(45, 48, 56));
                                egui::Image::new(spinner_image().clone()).paint_at(
                                    ui,
                                    Rect::from_center_size(rect.center(), vec2(64.0, 48.0)),
                                );
                            } else {
                                scene.show(ui, ui.available_size(), self.egui_d3d11);
                            }
                        }
                    },
                );
            });
    }

    fn allowed_in_windows(&self, tab: &mut Self::Tab) -> bool {
        !tab.is_fixed()
    }

    fn closeable(&mut self, tab: &mut Self::Tab) -> bool {
        !tab.is_fixed()
    }
}

pub trait DockStateExt<Tab> {
    fn find_tab(
        &self,
        predicate: impl Fn(&Tab) -> bool,
    ) -> Option<(SurfaceIndex, NodeIndex, TabIndex)>;
}

impl<Tab> DockStateExt<Tab> for DockState<Tab> {
    fn find_tab(
        &self,
        predicate: impl Fn(&Tab) -> bool,
    ) -> Option<(SurfaceIndex, NodeIndex, TabIndex)> {
        for (si, surface) in self.iter_surfaces().enumerate() {
            for (ni, node) in surface.iter_nodes().enumerate() {
                for (ti, tab) in node.iter_tabs().enumerate() {
                    if predicate(tab) {
                        return Some((si.into(), ni.into(), ti.into()));
                    }
                }
            }
        }

        None
    }
}
