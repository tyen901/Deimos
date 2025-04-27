use std::fmt::Display;

use deimos_data::{map::SBubbleParent, tfx::TfxFeatureRenderer};
use deimos_render::{camera::Camera, object::RenderObject, Renderer};
use egui::{vec2, Color32, Margin, Rect, RichText, TextEdit, Ui, Widget};
use egui_dock::{DockState, NodeIndex, SurfaceIndex, TabIndex};
use google_material_symbols::GoogleMaterialSymbols;
use tiger_parse::TigerReadable;
use tiger_pkg::{package_manager, TagHash};

use crate::ui::util::UiExt;

use super::{map_list::MapListTab, Tab, TabResult};

pub struct HomeTab;

impl HomeTab {
    pub fn ui(&self, ui: &mut Ui) -> TabResult {
        let mut result = TabResult::Continue;
        ui.add_space(32.0);
        ui.columns(2, |uis| {
            uis[0].heading("3D");
            uis[0].add_space(4.0);
            if uis[0]
                .d_button(format!("{} DYNAMICS", GoogleMaterialSymbols::DeployedCode))
                .clicked()
            {
                // self.added_nodes.push(Tab::DynamicList);
                result = TabResult::Open(Tab::DynamicList);
            }
            if uis[0]
                .d_button(format!("{} MAPS", GoogleMaterialSymbols::Map))
                .clicked()
            {
                result = TabResult::Open(Tab::MapList(MapListTab::new()));
            }
            uis[0].disable();
            let _ = uis[0].d_button(format!("{} STATICS", GoogleMaterialSymbols::Landscape));

            uis[1].heading("2D");
            uis[1].add_space(4.0);
            uis[1].disable();
            let _ = uis[1].d_button(format!("{} TEXTURES", GoogleMaterialSymbols::Image));
            let _ = uis[1].d_button(format!("{} UI", GoogleMaterialSymbols::DesktopWindows));
        });

        // ui.separator();

        // ui.with_layout(
        //     egui::Layout::top_down_justified(egui::Align::Center),
        //     |ui| {
        //         if ui
        //             .d_button(format!(
        //                 "{} Tag Lookup",
        //                 GoogleMaterialSymbols::Search
        //             ))
        //             .clicked()
        //         {
        //             self.added_nodes
        //                 .push(Tab::TagLookup(TagLookupTab::default()));
        //         }
        //     },
        // );

        result
    }
}
