use std::{collections::HashSet, fmt::Display, mem::Discriminant};

use egui::{vec2, Color32, Margin};
use egui_dock::{DockState, NodeIndex, SurfaceIndex, TabIndex};
use google_material_symbols::GoogleMaterialSymbols;

use crate::app::App;

use super::{scene3d::Scene, util::UiExt};

pub enum Tab {
    Home(Scene),
    Settings,
    Dynamics,
    Maps,
}

impl Tab {
    pub fn is_fixed(&self) -> bool {
        matches!(self, Tab::Home(_) | Tab::Settings)
    }

    /// Indicates whether the tab is unique. Only one instance of each unique tab can exist.
    pub fn is_unique(&self) -> bool {
        matches!(
            self,
            Tab::Home(_) | Tab::Settings | Tab::Dynamics | Tab::Maps
        )
    }
}

impl Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Tab::Settings => GoogleMaterialSymbols::Settings.to_string(),
            Tab::Home(_) => format!("{} HOME", GoogleMaterialSymbols::Home),
            Tab::Dynamics => format!("{} DYNAMICS", GoogleMaterialSymbols::DeployedCode),
            Tab::Maps => format!("{} MAPS", GoogleMaterialSymbols::Map),
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
                        Tab::Home(scene) => {
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
                                    self.added_nodes.push(Tab::Dynamics);
                                }
                                uis[0].disable();
                                let _ =
                                    uis[0].d_button(format!("{} MAPS", GoogleMaterialSymbols::Map));
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

                            scene.show(ui, vec2(ui.available_size().x, 850.0), self.egui_d3d11);
                        }
                        Tab::Settings => {
                            ui.weak("No settings are available");
                        }
                        Tab::Dynamics => {
                            // ui.weak("Wompy");
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(ui.cursor().min, ui.available_size()),
                                0,
                                Color32::RED,
                            );
                        }
                        Tab::Maps => {
                            // ui.weak("Wompy");
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(ui.cursor().min, ui.available_size()),
                                0,
                                Color32::RED,
                            );
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
