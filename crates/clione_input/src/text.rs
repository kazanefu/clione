use clione_core::InputEvent;

/// Represents the state of text input for a single frame.
#[derive(Debug, Default)]
pub struct TextInput {
    received: Vec<String>,
}

impl TextInput {
    /// Returns the text received during this frame.
    pub fn received(&self) -> &[String] {
        &self.received
    }

    pub(crate) fn push(&mut self, text: String) {
        self.received.push(text);
    }

    pub(crate) fn process_event(&mut self, event: &InputEvent) {
        if let InputEvent::Text(text) = event {
            self.push(text.clone());
        }
    }

    pub(crate) fn clear_frame(&mut self) {
        self.received.clear();
    }
}
