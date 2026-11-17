use std::collections::HashSet;

use clione_core::KeyCode;

/// Represents the state of keyboard input for a single frame.
#[derive(Debug, Default)]
pub struct KeyboardInput {
    /// The set of keys that are currently pressed.
    pressed: HashSet<KeyCode>,
    /// The set of keys that were pressed during this frame.
    just_pressed: HashSet<KeyCode>,
    /// The set of keys that were released during this frame.
    just_released: HashSet<KeyCode>,
}

impl KeyboardInput {
    /// Returns whether a key is currently pressed.
    pub fn pressed(&self, key: KeyCode) -> bool {
        self.pressed.contains(&key)
    }
    /// Returns whether a key was pressed during this frame.
    pub fn just_pressed(&self, key: KeyCode) -> bool {
        self.just_pressed.contains(&key)
    }
    /// Returns whether a key was released during this frame.
    pub fn just_released(&self, key: KeyCode) -> bool {
        self.just_released.contains(&key)
    }
    #[allow(dead_code)]
    pub(crate) fn press(&mut self, key: KeyCode) {
        if self.pressed.insert(key) {
            self.just_pressed.insert(key);
        }
    }
    #[allow(dead_code)]
    pub(crate) fn release(&mut self, key: KeyCode) {
        if self.pressed.remove(&key) {
            self.just_released.insert(key);
        }
    }
    #[allow(dead_code)]
    pub(crate) fn clear_frame(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }
}
