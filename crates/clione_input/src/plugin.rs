use clione_core::{App, Plugin};

use crate::keyboard::KeyboardInput;

/// A plugin that adds input handling to the application.
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(KeyboardInput::default());
    }
}
