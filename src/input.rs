use std::{collections::HashMap, rc::Rc, sync::Arc};

use glam::Vec2;
use sdl3::{event::Event, keyboard::Keycode};

pub type Key = Keycode;

#[derive(PartialEq, Eq, Default, Copy, Clone)]
pub enum ButtonState {
    #[default]
    Up,
    Down,
    #[doc(alias = "Repeated")]
    Held,
}

#[allow(unused)]
#[derive(PartialEq, Eq)]
#[repr(u8)]
pub enum MouseButton {
    #[doc(alias = "Mouse1")]
    Left = 1,
    #[doc(alias = "Mouse2")]
    Right = 2,
    #[doc(alias = "Mouse3")]
    Middle = 3,
    #[doc(alias = "Mouse4")]
    Back = 4,
    #[doc(alias = "Mouse5")]
    Forward = 5,
}

impl TryFrom<u8> for MouseButton {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            1 => Self::Left,
            2 => Self::Right,
            3 => Self::Middle,
            4 => Self::Back,
            5 => Self::Forward,
            _ => return Err(()),
        })
    }
}

impl TryFrom<sdl3::mouse::MouseButton> for MouseButton {
    type Error = ();
    fn try_from(value: sdl3::mouse::MouseButton) -> Result<Self, Self::Error> {
        Ok(match value {
            sdl3::mouse::MouseButton::Unknown => return Err(()),
            sdl3::mouse::MouseButton::Left => Self::Left,
            sdl3::mouse::MouseButton::Middle => Self::Middle,
            sdl3::mouse::MouseButton::Right => Self::Right,
            sdl3::mouse::MouseButton::X1 => Self::Back,
            sdl3::mouse::MouseButton::X2 => Self::Forward,
        })
    }
}

pub struct MouseKeyboardState {
    sdl: Rc<sdl3::Sdl>,
    sdl_mouse: sdl3::mouse::MouseUtil,
    window: Rc<sdl3::video::Window>,
    pub last_mouse_pos: Vec2,
    lock_mouse_pos: Vec2,
    pub mouse_delta: Vec2,
    pub mousewheel_delta: f32,

    keys: HashMap<Key, ButtonState>,

    /// Left mouse button
    mouse1: ButtonState,
    /// Right mouse button
    mouse2: ButtonState,
    /// Scroll wheel button
    mouse3: ButtonState,
    /// 'Back' side button
    mouse4: ButtonState,
    /// 'Forward' side button
    mouse5: ButtonState,
}

impl MouseKeyboardState {
    pub fn new(sdl: Rc<sdl3::Sdl>, window: Rc<sdl3::video::Window>) -> Self {
        Self {
            sdl_mouse: sdl.mouse(),
            sdl,
            window,
            last_mouse_pos: Vec2::ZERO,
            lock_mouse_pos: Vec2::ZERO,
            mouse_delta: Vec2::ZERO,
            mousewheel_delta: 0.0,
            keys: Default::default(),
            mouse1: ButtonState::Up,
            mouse2: ButtonState::Up,
            mouse3: ButtonState::Up,
            mouse4: ButtonState::Up,
            mouse5: ButtonState::Up,
        }
    }
}

#[allow(unused)]
impl MouseKeyboardState {
    /// Handles window events and updates the state accordingly
    pub fn handle_event(
        &mut self,
        event: &sdl3::event::Event,
        handle_keyboard: bool,
        handle_mouse: bool,
    ) {
        match event {
            Event::KeyDown {
                keycode: Some(vk),
                // keymod,
                repeat,
                ..
            } => {
                if !handle_keyboard {
                    return;
                }
                let key = self.key_state_mut(*vk);
                *key = if *repeat {
                    ButtonState::Held
                } else {
                    ButtonState::Down
                };
            }
            Event::KeyUp {
                keycode: Some(vk), ..
            } => {
                if !handle_keyboard {
                    return;
                }
                let key = self.key_state_mut(*vk);
                *key = ButtonState::Up;
            }
            Event::MouseButtonDown { mouse_btn, .. } | Event::MouseButtonUp { mouse_btn, .. } => {
                if !handle_mouse {
                    return;
                }
                let button = MouseButton::try_from(*mouse_btn).unwrap();
                let state = match event {
                    Event::MouseButtonDown { .. } => ButtonState::Down,
                    Event::MouseButtonUp { .. } => ButtonState::Up,
                    _ => unreachable!(),
                };
                match button {
                    MouseButton::Left => self.mouse1 = state,
                    MouseButton::Right => self.mouse2 = state,
                    MouseButton::Middle => self.mouse3 = state,
                    MouseButton::Back => self.mouse4 = state,
                    MouseButton::Forward => self.mouse5 = state,
                }
            }
            sdl3::event::Event::MouseMotion {
                xrel, yrel, x, y, ..
            } => {
                if !handle_mouse {
                    return;
                }
                self.mouse_delta = Vec2::new(*xrel, *yrel);
                if self.get_relative_mouse_mode() {
                    self.sdl_mouse.warp_mouse_in_window(
                        &self.window,
                        self.lock_mouse_pos.x,
                        self.lock_mouse_pos.y,
                    );
                    self.sdl_mouse.show_cursor(false);
                }
                self.last_mouse_pos = Vec2::new(*x as f32, *y as f32);
            }
            sdl3::event::Event::MouseWheel { y, .. } => {
                if !handle_mouse {
                    return;
                }
                self.mousewheel_delta = *y;
            }
            _ => {}
        }
    }

    fn next_button_state(new_state: ButtonState, current: ButtonState) -> ButtonState {
        match new_state {
            ButtonState::Down => match current {
                ButtonState::Up => ButtonState::Down,
                ButtonState::Down => ButtonState::Held,
                ButtonState::Held => ButtonState::Held,
            },
            ButtonState::Up => ButtonState::Up,
            ButtonState::Held => ButtonState::Held,
        }
    }

    /// Call this at the end of the frame to update the state of keys from Down to Held
    pub fn update_keystates(&mut self) {
        for (_, state) in self.keys.iter_mut() {
            if *state == ButtonState::Down {
                *state = ButtonState::Held;
            }
        }

        if self.mouse1 == ButtonState::Down {
            self.mouse1 = ButtonState::Held;
        }
        if self.mouse2 == ButtonState::Down {
            self.mouse2 = ButtonState::Held;
        }
        if self.mouse3 == ButtonState::Down {
            self.mouse3 = ButtonState::Held;
        }
        if self.mouse4 == ButtonState::Down {
            self.mouse4 = ButtonState::Held;
        }
        if self.mouse5 == ButtonState::Down {
            self.mouse5 = ButtonState::Held;
        }

        self.mouse_delta = Vec2::ZERO;
        self.mousewheel_delta = 0.0;
    }

    pub fn set_relative_mouse_mode(&mut self, capture: bool) {
        if capture != self.get_relative_mouse_mode() {
            if capture {
                self.lock_mouse_pos = self.last_mouse_pos;
            }

            self.sdl
                .mouse()
                .set_relative_mouse_mode(&self.window, capture);
        }
    }

    pub fn get_relative_mouse_mode(&self) -> bool {
        self.sdl.mouse().relative_mouse_mode(&self.window)
    }
}

impl MouseKeyboardState {
    pub fn key_state(&self, vk: Key) -> ButtonState {
        self.keys.get(&vk).copied().unwrap_or(ButtonState::Up)
    }

    pub fn key_state_mut(&mut self, vk: Key) -> &mut ButtonState {
        self.keys.entry(vk).or_insert(ButtonState::Up)
    }

    /// Returns true if the key is being held.
    pub fn is_key_down(&self, vk: Key) -> bool {
        matches!(self.key_state(vk), ButtonState::Down | ButtonState::Held)
    }

    /// Returns true if the key was pressed (went from up to down)
    pub fn is_key_pressed(&self, vk: Key) -> bool {
        self.key_state(vk) == ButtonState::Down
    }

    pub fn ctrl(&self) -> bool {
        self.is_key_down(Key::LCtrl) || self.is_key_down(Key::RCtrl)
    }

    pub fn alt(&self) -> bool {
        self.is_key_down(Key::LAlt) || self.is_key_down(Key::RAlt)
    }

    pub fn shift(&self) -> bool {
        self.is_key_down(Key::LShift) || self.is_key_down(Key::RShift)
    }

    pub fn mouse_button_state(&self, button: MouseButton) -> ButtonState {
        match button {
            MouseButton::Left => self.mouse1,
            MouseButton::Right => self.mouse2,
            MouseButton::Middle => self.mouse3,
            MouseButton::Forward => self.mouse5,
            MouseButton::Back => self.mouse4,
        }
    }

    pub fn is_mouse_button_clicked(&self, button: MouseButton) -> bool {
        self.mouse_button_state(button) == ButtonState::Down
    }

    pub fn is_mouse_button_down(&self, button: MouseButton) -> bool {
        matches!(
            self.mouse_button_state(button),
            ButtonState::Down | ButtonState::Held
        )
    }

    pub fn mouse_left_clicked(&self) -> bool {
        self.is_mouse_button_clicked(MouseButton::Left)
    }

    pub fn mouse_right_clicked(&self) -> bool {
        self.is_mouse_button_clicked(MouseButton::Right)
    }

    pub fn mouse_left(&self) -> bool {
        self.is_mouse_button_down(MouseButton::Left)
    }

    pub fn mouse_right(&self) -> bool {
        self.is_mouse_button_down(MouseButton::Right)
    }

    pub fn mouse_middle(&self) -> bool {
        self.is_mouse_button_down(MouseButton::Middle)
    }

    pub fn mouse_forward(&self) -> bool {
        self.is_mouse_button_down(MouseButton::Forward)
    }

    pub fn mouse_back(&self) -> bool {
        self.is_mouse_button_down(MouseButton::Back)
    }
}
