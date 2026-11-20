/// An input mode for the terminal.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum InputMode {
    /// Receives input immediately without waiting for a newline.
    #[default]
    Raw,
    /// Receives text input after a line is submitted.
    Line,
}

/// A key code mode for the terminal.
#[derive(Default, Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyCodeSupport {
    /// The terminal supports retrieving physical key codes.
    Supported,
    /// The terminal does not support retrieving physical key codes.
    Unsupported,
    /// The availability of key codes is unknown.
    #[default]
    Unknown,
}

/// A terminal capability for input handling.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct TerminalCapabilities {
    /// Whether the terminal supports physical key codes.
    pub key_code: KeyCodeSupport,
}

/// `Resource` settings for input handling in the terminal.
#[derive(Debug, Default, Eq, PartialEq, Clone, Copy)]
pub struct InputSettings {
    /// The input mode for the terminal.
    pub mode: InputMode,
    /// The terminal capabilities for input handling.
    pub capabilities: TerminalCapabilities,
}
