use std::{collections::HashSet, fmt::Display, mem::Discriminant};

use egui::Margin;
use egui_dock::{DockState, NodeIndex, SurfaceIndex, TabIndex};
use google_material_symbols::GoogleMaterialSymbols;

use super::util::UiExt;

pub enum Tab {
    Home,
    Settings,
    Dynamics,
}

impl Tab {
    pub fn is_fixed(&self) -> bool {
        matches!(self, Tab::Home | Tab::Settings)
    }

    /// Indicates whether the tab is unique. Only one instance of each unique tab can exist.
    pub fn is_unique(&self) -> bool {
        matches!(self, Tab::Home | Tab::Settings | Tab::Dynamics)
    }
}

impl Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Tab::Settings => GoogleMaterialSymbols::Settings.to_string(),
            Tab::Home => format!("{} HOME", GoogleMaterialSymbols::Home),
            Tab::Dynamics => format!("{} DYNAMICS", GoogleMaterialSymbols::DeployedCode),
        };

        f.write_str(&s)
    }
}

pub struct TabViewer<'a> {
    pub added_nodes: &'a mut Vec<Tab>,
}

impl<'a> egui_dock::TabViewer for TabViewer<'a> {
    type Tab = Tab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.to_string().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        egui::Frame::new()
            .outer_margin(Margin::same(127))
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
                                    self.added_nodes.push(Tab::Dynamics);
                                }
                                uis[0].disable();
                                uis[0].d_button(format!(
                                    "{} STATICS",
                                    GoogleMaterialSymbols::Landscape
                                ));
                                uis[0].d_button(format!("{} MAPS", GoogleMaterialSymbols::Map));

                                uis[1].heading("2D");
                                uis[1].add_space(4.0);
                                uis[1].disable();
                                uis[1]
                                    .d_button(format!("{} TEXTURES", GoogleMaterialSymbols::Image));
                                uis[1].d_button(format!(
                                    "{} UI",
                                    GoogleMaterialSymbols::DesktopWindows
                                ));
                            });
                        }
                        Tab::Settings => {
                            ui.weak("No settings are available");
                        }
                        Tab::Dynamics => {
                            ui.weak("Wompy");
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
            if let Some((ti, (ni, _tab))) = surface
                .iter_all_tabs()
                .enumerate()
                .find(|&(_ti, (_ni, tab))| predicate(tab))
            {
                return Some((si.into(), ni, ti.into()));
            }
        }

        None
    }
}
