/// A tick-oriented queue of messages of one type.
#[derive(Debug)]
pub struct Messages<T> {
    items: Vec<T>,
}

impl<T> Default for Messages<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T> Messages<T> {
    /// Appends a message to the current queue.
    pub fn write(&mut self, message: T) {
        self.items.push(message);
    }

    /// Appends a message using the legacy event terminology.
    pub fn send(&mut self, message: T) {
        self.write(message);
    }

    /// Iterates over queued messages without consuming them.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    /// Removes and returns all queued messages.
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.items.drain(..)
    }

    /// Removes all queued messages.
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

/// Legacy name retained for compatibility.
pub type Events<T> = Messages<T>;

/// Tracks an independent read position in a message queue.
#[derive(Debug)]
pub struct MessageReader<'a, T> {
    messages: &'a Messages<T>,
    cursor: &'a mut usize,
}

impl<'a, T> MessageReader<'a, T> {
    pub(crate) fn from_state(messages: &'a Messages<T>, cursor: &'a mut usize) -> Self {
        Self { messages, cursor }
    }

    /// Reads messages that have not been seen by this reader.
    pub fn read(&mut self) -> impl Iterator<Item = &'a T> {
        if *self.cursor > self.messages.items.len() {
            *self.cursor = 0;
        }
        let start = *self.cursor;
        *self.cursor = self.messages.items.len();
        self.messages.items[start..].iter()
    }
}

/// A writer for a typed message queue.
pub struct MessageWriter<'a, T> {
    messages: &'a mut Messages<T>,
}

impl<'a, T> MessageWriter<'a, T> {
    /// Creates a writer over a message queue.
    pub fn new(messages: &'a mut Messages<T>) -> Self {
        Self { messages }
    }

    /// Writes one message.
    pub fn write(&mut self, message: T) {
        self.messages.write(message);
    }
}
