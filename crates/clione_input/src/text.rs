use clione_core::InputEvent;

/// Represents the state of text input for a single frame.
#[derive(Debug, Default)]
pub struct TextInput {
    received: String,
    just_received: bool,
}

impl TextInput {
    /// Returns the text received during this frame, if any.
    pub fn just_received(&self) -> Option<&str> {
        if self.just_received {
            Some(&self.received)
        } else {
            None
        }
    }
    /// Returns the most recently received text.
    pub fn received(&self) -> &str {
        &self.received
    }

    pub(crate) fn push(&mut self, text: &str) {
        self.received.push_str(text);
    }

    pub(crate) fn process_event(&mut self, event: &InputEvent) {
        if let InputEvent::Text(text) = event {
            if !self.just_received {
                self.received.clear();
            }
            self.just_received = true;
            self.push(&text);
        }
    }

    pub(crate) fn clear_frame(&mut self) {
        self.just_received = false;
    }
}
