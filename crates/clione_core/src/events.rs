/// A tick-oriented queue of events of one type.
pub struct Events<T> {
    items: Vec<T>,
}

impl<T> Default for Events<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T> Events<T> {
    /// Appends an event to the current event queue.
    pub fn send(&mut self, event: T) {
        self.items.push(event);
    }

    /// Iterates over queued events without consuming them.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    /// Removes and returns all queued events.
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.items.drain(..)
    }

    /// Removes all queued events.
    pub fn clear(&mut self) {
        self.items.clear();
    }
}
