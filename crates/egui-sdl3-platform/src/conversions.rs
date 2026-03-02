use egui::Key;

/// A trait that adds a method to convert to an egui key
pub trait ToEguiKey {
    /// Convert the struct to an egui key
    fn to_egui_key(&self) -> Option<egui::Key>;
}

impl ToEguiKey for sdl3::keyboard::Keycode {
    fn to_egui_key(&self) -> Option<egui::Key> {
        Some(match *self {
            Self::Left => Key::ArrowLeft,
            Self::Up => Key::ArrowUp,
            Self::Right => Key::ArrowRight,
            Self::Down => Key::ArrowDown,
            Self::Escape => Key::Escape,
            Self::Tab => Key::Tab,
            Self::Backspace => Key::Backspace,
            Self::Space => Key::Space,
            Self::Return => Key::Enter,
            Self::Insert => Key::Insert,
            Self::Home => Key::Home,
            Self::Delete => Key::Delete,
            Self::End => Key::End,
            Self::PageDown => Key::PageDown,
            Self::PageUp => Key::PageUp,
            Self::Kp0 | Self::_0 => Key::Num0,
            Self::Kp1 | Self::_1 => Key::Num1,
            Self::Kp2 | Self::_2 => Key::Num2,
            Self::Kp3 | Self::_3 => Key::Num3,
            Self::Kp4 | Self::_4 => Key::Num4,
            Self::Kp5 | Self::_5 => Key::Num5,
            Self::Kp6 | Self::_6 => Key::Num6,
            Self::Kp7 | Self::_7 => Key::Num7,
            Self::Kp8 | Self::_8 => Key::Num8,
            Self::Kp9 | Self::_9 => Key::Num9,
            Self::A => Key::A,
            Self::B => Key::B,
            Self::C => Key::C,
            Self::D => Key::D,
            Self::E => Key::E,
            Self::F => Key::F,
            Self::G => Key::G,
            Self::H => Key::H,
            Self::I => Key::I,
            Self::J => Key::J,
            Self::K => Key::K,
            Self::L => Key::L,
            Self::M => Key::M,
            Self::N => Key::N,
            Self::O => Key::O,
            Self::P => Key::P,
            Self::Q => Key::Q,
            Self::R => Key::R,
            Self::S => Key::S,
            Self::T => Key::T,
            Self::U => Key::U,
            Self::V => Key::V,
            Self::W => Key::W,
            Self::X => Key::X,
            Self::Y => Key::Y,
            Self::Z => Key::Z,
            _ => {
                return None;
            }
        })
    }
}
