use std::{collections::BTreeMap, rc::Rc, sync::Arc};

use deimos_render::{gpu::command_list::CommandList, Gpu};
use egui::{Color32, FontId};
use egui_dock::{DockArea, DockState};
use google_material_symbols::GoogleMaterialSymbols;
use tabs::{Tab, TabViewer};

pub mod tabs;
pub mod util;

pub struct Gui {
    window: Rc<sdl3::video::Window>,
    sdl: Rc<sdl3::Sdl>,

    pub egui_d3d11: egui_d3d11::D3D11Renderer,
    pub egui_sdl3: egui_sdl3_platform::Platform,
    tree: DockState<Tab>,
}

impl Gui {
    pub fn new(
        gpu: &Gpu,
        sdl: Rc<sdl3::Sdl>,
        window: Rc<sdl3::video::Window>,
    ) -> anyhow::Result<Self> {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "ppfraktionmono".into(),
            Arc::new(egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/ppfraktionmono-regular.otf"
            ))),
        );
        fonts.font_data.insert(
            "ppfraktionmono-bold".into(),
            Arc::new(egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/ppfraktionmono-bold.otf"
            ))),
        );
        fonts.font_data.insert(
            "marathonshapiro_wide".into(),
            Arc::new(egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/marathonshapiro_wide65.otf"
            ))),
        );
        fonts.font_data.insert(
            "khinterference-regular".into(),
            Arc::new(egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/khinterference-regular.otf"
            ))),
        );
        fonts.font_data.insert(
            "MaterialSymbolsRounded-Medium".into(),
            Arc::new(egui::FontData::from_static(
                GoogleMaterialSymbols::FONT_BYTES,
            )),
        );

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "ppfraktionmono".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(1, "ppfraktionmono-bold".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(2, "MaterialSymbolsRounded-Medium".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "ppfraktionmono".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Name("khinterference-regular".into()))
            .or_default()
            .insert(0, "khinterference-regular".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Name("khinterference-regular".into()))
            .or_default()
            .insert(1, "MaterialSymbolsRounded-Medium".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Name("shapiro".into()))
            .or_default()
            .insert(0, "marathonshapiro_wide".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Name("shapiro".into()))
            .or_default()
            .insert(1, "MaterialSymbolsRounded-Medium".to_owned());

        let egui_sdl3 = egui_sdl3_platform::Platform::new(gpu.swapchain_resolution())?;
        egui_sdl3.context().set_fonts(fonts);
        egui_sdl3.context().style_mut(|s| {
            s.visuals.override_text_color = Some(Color32::WHITE);
            s.spacing.button_padding = egui::vec2(30.0, 20.0);
            s.spacing.item_spacing = egui::vec2(20.0, 10.0);
        });

        // Redefine text_styles
        let text_styles: BTreeMap<_, _> = [
            (
                egui::TextStyle::Heading,
                FontId::new(42.0, egui::FontFamily::Name("shapiro".into())),
            ),
            (
                egui::TextStyle::Body,
                FontId::new(18.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Monospace,
                FontId::new(14.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Button,
                // FontId::new(20.0, egui::FontFamily::Monospace),
                FontId::new(
                    20.0,
                    egui::FontFamily::Name("khinterference-regular".into()),
                ),
            ),
            (
                egui::TextStyle::Small,
                FontId::new(10.0, egui::FontFamily::Proportional),
            ),
        ]
        .into();

        // Mutate global styles with new text styles
        egui_sdl3
            .context()
            .all_styles_mut(move |style| style.text_styles = text_styles.clone());

        let tree = DockState::new(vec![Tab::Settings, Tab::Home]);

        Ok(Self {
            window,
            sdl,
            egui_d3d11: egui_d3d11::D3D11Renderer::new(&gpu)?,
            egui_sdl3,
            tree,
        })
    }

    pub fn draw(&mut self, cmd: &mut CommandList) {
        let mut ctx = self
            .egui_sdl3
            .begin_frame(self.window.size(), self.window.display_scale());
        // egui::Window::new("Demo Window").show(&ctx, |ui| {
        //     ui.label("Hello, World!");
        // });
        ctx.style_mut(|s| s.visuals.panel_fill = Color32::from_black_alpha(96));
        DockArea::new(&mut self.tree)
            .style(egui_dock::Style::from_egui(ctx.style().as_ref()))
            .show_leaf_collapse_buttons(false)
            .show(&ctx, &mut TabViewer);
        let output = self
            .egui_sdl3
            .end_frame(&mut self.sdl.video().unwrap())
            .unwrap();
        self.egui_d3d11.paint(cmd, output, &mut ctx);
    }
}
