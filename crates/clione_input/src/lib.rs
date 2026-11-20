mod keyboard;
mod plugin;
mod settings;
mod text;

pub use keyboard::{KeyboardInput, KeyIdentifier};
pub use plugin::InputPlugin;
pub use settings::{InputMode, InputSettings, KeyCodeSupport, TerminalCapabilities};
pub use text::TextInput;

#[cfg(test)]
mod tests;
