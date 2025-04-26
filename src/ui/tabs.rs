use std::fmt::Display;

use egui::Margin;
use google_material_symbols::GoogleMaterialSymbols;

use super::util::UiExt;

pub enum Tab {
    Home,
    Settings,
}

impl Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Tab::Home => format!("{} HOME", GoogleMaterialSymbols::Home),
            Tab::Settings => GoogleMaterialSymbols::Settings.to_string(),
        };

        f.write_str(&s)
    }
}

pub struct TabViewer;

impl egui_dock::TabViewer for TabViewer {
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
                            ui.with_layout(
                                egui::Layout::left_to_right(egui::Align::Min)
                                    .with_main_justify(true),
                                |ui| {
                                    ui.d_button("MAPS");
                                },
                            );
                            ui.add_space(32.0);
                            ui.columns(3, |uis| {
                                uis[0].heading("MODELS");
                                uis[0].add_space(4.0);
                                uis[0].d_button("API");
                                uis[0].d_button("DYNAMICS");
                                uis[0].d_button("STATICS");

                                uis[1].heading("AUDIO");
                                uis[1].add_space(4.0);
                                uis[1].d_button("ALL SOUNDS");
                                uis[1].d_button("WEAPON AUDIO");

                                uis[2].heading("OTHER");
                                uis[2].add_space(4.0);
                                uis[2].d_button("STRINGS");
                                uis[2].d_button("TEXTURES");
                                uis[2].d_button("MATERIALS");
                                uis[2].d_button("COLLECTIONS");
                            });
                        }
                        Tab::Settings => {
                            ui.label("Hi");
                        }
                    },
                );
            });
    }

    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        false
    }

    fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
        false
    }
}
