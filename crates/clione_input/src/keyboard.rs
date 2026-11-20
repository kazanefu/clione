use std::collections::HashSet;

use clione_core::{ButtonState, InputEvent, InputState, Key, KeyCode};

/// Represents the state of keyboard input for a single frame.
#[derive(Debug, Default)]
pub struct KeyboardInput {
    /// The set of keys that are currently pressed.
    pressed_codes: HashSet<KeyCode>,
    /// The set of keys that were pressed during this frame.
    just_pressed_codes: HashSet<KeyCode>,
    /// The set of keys that were released during this frame.
    just_released_codes: HashSet<KeyCode>,
    /// The set of keys that are currently pressed.
    pressed: HashSet<Key>,
    /// The set of keys that were pressed during this frame.
    just_pressed: HashSet<Key>,
    /// The set of keys that were released during this frame.
    just_released: HashSet<Key>,
}

/// A trait for types that can be used to identify keys in the `KeyboardInput` struct. This trait allows for checking the state of keys using different key representations, such as `KeyCode` and `Key`.
pub trait KeyIdentifier {
    fn pressed(&self, keyboard_input: &KeyboardInput) -> bool;
    fn just_pressed(&self, keyboard_input: &KeyboardInput) -> bool;
    fn just_released(&self, keyboard_input: &KeyboardInput) -> bool;

    fn state(&self, input: &KeyboardInput) -> InputState {
        if self.just_pressed(input) {
            InputState::JustPressed
        } else if self.just_released(input) {
            InputState::JustReleased
        } else if self.pressed(input) {
            InputState::Pressed
        } else {
            InputState::Released
        }
    }
}

impl KeyIdentifier for KeyCode {
    fn pressed(&self, keyboard_input: &KeyboardInput) -> bool {
        keyboard_input.pressed_codes.contains(self)
    }
    fn just_pressed(&self, keyboard_input: &KeyboardInput) -> bool {
        keyboard_input.just_pressed_codes.contains(self)
    }
    fn just_released(&self, keyboard_input: &KeyboardInput) -> bool {
        keyboard_input.just_released_codes.contains(self)
    }
}

impl KeyIdentifier for Key {
    fn pressed(&self, keyboard_input: &KeyboardInput) -> bool {
        keyboard_input.pressed.contains(self)
    }
    fn just_pressed(&self, keyboard_input: &KeyboardInput) -> bool {
        keyboard_input.just_pressed.contains(self)
    }
    fn just_released(&self, keyboard_input: &KeyboardInput) -> bool {
        keyboard_input.just_released.contains(self)
    }
}

impl KeyboardInput {
    /// Returns the state of a key.
    pub fn get<T: KeyIdentifier>(&self, key: T) -> InputState {
        key.state(self)
    }
    /// Returns whether a key is currently pressed.
    pub fn pressed<T: KeyIdentifier>(&self, key: T) -> bool {
        key.pressed(self)
    }
    /// Returns whether a key was pressed during this frame.
    pub fn just_pressed<T: KeyIdentifier>(&self, key: T) -> bool {
        key.just_pressed(self)
    }
    /// Returns whether a key was released during this frame.
    pub fn just_released<T: KeyIdentifier>(&self, key: T) -> bool {
        key.just_released(self)
    }
    pub(crate) fn press_code(&mut self, key: Option<KeyCode>) {
        let Some(key) = key else {
            return;
        };
        if self.pressed_codes.insert(key) {
            self.just_released_codes.remove(&key);
            self.just_pressed_codes.insert(key);
        }
    }

    pub(crate) fn release_code(&mut self, key: Option<KeyCode>) {
        let Some(key) = key else {
            return;
        };
        if self.pressed_codes.remove(&key) {
            self.just_pressed_codes.remove(&key);
            self.just_released_codes.insert(key);
        }
    }

    pub(crate) fn press(&mut self, key: Key) {
        if self.pressed.insert(key) {
            self.just_released.remove(&key);
            self.just_pressed.insert(key);
        }
    }

    pub(crate) fn release(&mut self, key: Key) {
        if self.pressed.remove(&key) {
            self.just_pressed.remove(&key);
            self.just_released.insert(key);
        }
    }

    /// Clears transient key states.
    pub(crate) fn clear_frame(&mut self) {
        self.just_pressed_codes.clear();
        self.just_released_codes.clear();
        self.just_pressed.clear();
        self.just_released.clear();
    }

    pub(crate) fn process_event(&mut self, event: &InputEvent) {
        match event {
            InputEvent::Keyboard(keyboard) => match keyboard.state {
                ButtonState::Pressed => {
                    self.press(keyboard.key);
                    self.press_code(keyboard.key_code);
                }
                ButtonState::Released => {
                    self.release_code(keyboard.key_code);
                    self.release(keyboard.key);
                }
            },
            InputEvent::Text(_) | InputEvent::Resize(_) | InputEvent::Quit => {}
        }
    }
}
