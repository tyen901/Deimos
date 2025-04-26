use egui::{Color32, Response, RichText};

pub trait UiExt {
    #[must_use]
    fn d_button(&mut self, text: impl Into<RichText>) -> Response;
}

impl UiExt for egui::Ui {
    fn d_button(&mut self, text: impl Into<RichText>) -> Response {
        let r = self
            .add(
                egui::Button::new(text.into().color(Color32::BLACK))
                    .min_size(egui::vec2(120.0, 60.0))
                    .corner_radius(8)
                    .fill(Color32::WHITE),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);

        if r.hovered() {
            self.painter()
                .rect_filled(r.rect, 8, Color32::from_black_alpha(48));
        }

        r
    }
}
