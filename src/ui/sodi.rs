use std::time::Instant;

use deimos_render::util::math::FloatExt;
use egui::*;
use google_material_symbols::GoogleMaterialSymbols;
use windows_registry::CURRENT_USER;

pub struct SodiVow {
    accepted: bool,
    start_time: Instant,
    /// Used to reset timer when drawing starts
    first_draw: bool,
    open: bool,
}

impl SodiVow {
    const REG_DEIMOS: &str = "Software\\Deimos\\";

    /// Disables the popup permanently
    fn set_accepted(&mut self) {
        self.accepted = true;
        if let Ok(key) = CURRENT_USER.create(Self::REG_DEIMOS) {
            key.set_u32("VowAccepted", 1).ok();
        }
        self.open = false;
    }

    fn already_accepted() -> bool {
        match CURRENT_USER.create(Self::REG_DEIMOS) {
            Ok(key) => key.get_u32("VowAccepted").unwrap_or_default() != 0,
            Err(e) => {
                error!("Failed to check SODI vow acceptance: {e:?}");
                false
            }
        }
    }
}

impl Default for SodiVow {
    fn default() -> Self {
        let already_accepted = Self::already_accepted();
        Self {
            accepted: already_accepted,
            start_time: Instant::now(),
            first_draw: true,
            open: !already_accepted,
        }
    }
}

impl SodiVow {
    pub fn draw(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        if self.first_draw {
            self.start_time = Instant::now();
            self.first_draw = false;
        }

        // Input catcher
        egui::Area::new("sodi_input_catcher".into())
            .order(Order::Middle)
            .default_size(ctx.viewport_rect().size())
            .movable(false)
            .show(ctx, |ui| {
                ui.allocate_exact_size(ui.available_size(), egui::Sense::all());
            });

        // 0s - 1s fade in black background
        // 0.5s - 1.5s fade in frame
        // 1s - 16s timer
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let button_time = elapsed.remap_clamped(1., 16., 15., 0.);

        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Middle,
            Id::new("update_channel_selector_bg").with("layer"),
        ))
        .rect_filled(
            egui::Rect::EVERYTHING,
            CornerRadius::default(),
            Color32::from_black_alpha(elapsed.remap_clamped(0.25, 1.0, 0.0, 196.0) as u8),
        );

        egui::Area::new(egui::Id::new("Sodi"))
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .default_width(1200.0)
            .show(ctx, |ui| {
                egui::Frame::window(&ctx.style())
                    .inner_margin(32.0)
                    .multiply_with_opacity(elapsed.remap_clamped(0.25, 1.0, 0.0, 1.0))
                    .show(ui, |ui| {
                        ui.style_mut().spacing.item_spacing = vec2(42.0, 36.0);
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.style_mut().spacing.item_spacing.y = 12.0;
                                ui.label(
                                    RichText::new(GoogleMaterialSymbols::VisibilityLock.to_string())
                                        .color(Color32::WHITE)
                                        .size(128.0),
                                );
                            });

                            ui.vertical(|ui| {
                                ui.style_mut().spacing.item_spacing.y = 12.0;
                                ui.heading(
                                    egui::RichText::new("For your eyes only")
                                        .color(Color32::WHITE)
                                        .strong()
                                        .size(36.0),
                                );
                                ui.add_space(12.0);

                                #[rustfmt::skip]
                                ui.label(
"Deimos is a tool for artists to use as reference, and while it may seem tempting to use it for exploring unreleased content, you should think twice before doing so, and thrice before sharing leaks with others.

I developed this tool to better understand Marathon’s renderer and VFX system, and that won't change.

I do not condone the use of Deimos for leaks, spoilers, or any actions that violate Bungie’s Terms of Service.

Using Deimos to leak content will reduce the likelihood of future public releases, and I may stop its development altogether at any time if misuse continues.

Don't ruin the secrets of the game for yourself or others.
",
                                );
                                ui.strong("By using Deimos, you agree to keep unreleased content to your eyes only.");

                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut self.accepted, RichText::new("I vow to not use Deimos for leaking content").color(Color32::GRAY));

                                    if button_time > 0. {
                                        ui.add_enabled_ui(false, |ui| {
                                            ui.button(format!("Wait {button_time:.0}s..."))
                                        });
                                    } else {
                                        ui.add_enabled_ui(self.accepted, |ui| {
                                            if ui.button("I Accept").clicked() {
                                                self.set_accepted();
                                            }
                                        });
                                    }
                                });
                            });
                        });
                    });
            });
    }
}
