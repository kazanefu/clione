use crate::App;

/// A reusable bundle of application configuration and systems.
pub trait Plugin {
    /// Registers the plugin's resources and systems on the application.
    fn build(&self, app: &mut App);
}
