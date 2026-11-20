/// A buffer of messages with a starting message index.
#[derive(Clone, Debug, Eq, PartialEq)]
struct MessageBuffer<T> {
    items: Vec<T>,
    start_count: usize,
}

impl<T> Default for MessageBuffer<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            start_count: 0,
        }
    }
}

impl<T> MessageBuffer<T> {
    pub(crate) fn clear(&mut self, start_count: usize) {
        self.items.clear();
        self.start_count = start_count;
    }
}

/// A tick-oriented queue of messages of one type.
#[derive(Debug)]
pub struct Messages<T> {
    buffers: [MessageBuffer<T>; 2],
    current: usize,
    message_count: usize,
}

impl<T> Default for Messages<T> {
    fn default() -> Self {
        Self {
            buffers: [MessageBuffer::default(), MessageBuffer::default()],
            current: 0,
            message_count: 0,
        }
    }
}

impl<T> Messages<T> {
    /// Appends a message to the current queue.
    pub fn write(&mut self, message: T) {
        self.buffers[self.current].items.push(message);
        self.message_count += 1;
    }

    /// Advances the message queue to the next tick, clearing messages from two ticks ago.
    pub fn update(&mut self) {
        self.current = 1 - self.current;

        let buffer = &mut self.buffers[self.current];
        buffer.clear(self.message_count);
        buffer.start_count = self.message_count;
    }

    /// Appends a message using the legacy event terminology.
    pub fn send(&mut self, message: T) {
        self.write(message);
    }

    /// Iterates over queued messages from a specific buffer without consuming them.
    fn iter_from(&self, index: usize, start: usize, end: usize) -> impl Iterator<Item = &T> {
        let buffer = &self.buffers[index];

        let len = buffer.items.len();

        let start_index = start.saturating_sub(buffer.start_count).min(len);

        let end_index = end
            .saturating_sub(buffer.start_count)
            .clamp(start_index, len);

        buffer.items[start_index..end_index].iter()
    }

    /// Iterates over all currently retained messages without consuming them.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.iter_from_range(0, self.message_count)
    }

    /// Iterates over retained messages within the specified message index range.
    ///
    /// The range is half-open (`start..end`). Messages that are no longer
    /// retained in the queue are not returned.
    pub fn iter_from_range(&self, start: usize, end: usize) -> impl Iterator<Item = &T> {
        let current = self.current;
        let previous = 1 - current;
        self.iter_from(previous, start, end)
            .chain(self.iter_from(current, start, end))
    }

    /// Removes and returns all retained messages, oldest first.
    ///
    /// Drained messages are no longer available to message readers.
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        let previous = 1 - self.current;
        let current = self.current;
        let message_count = self.message_count;

        // Reset the index of both buffers so subsequent writes remain aligned.
        self.buffers[previous].start_count = message_count;
        self.buffers[current].start_count = message_count;

        let (a, b) = self.buffers.split_at_mut(1);

        let (previous_items, current_items) = if previous == 0 {
            (&mut a[0].items, &mut b[0].items)
        } else {
            (&mut b[0].items, &mut a[0].items)
        };

        previous_items.drain(..).chain(current_items.drain(..))
    }

    /// Removes all queued messages.
    pub fn clear(&mut self) {
        for buffer in &mut self.buffers {
            buffer.clear(self.message_count);
        }
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

    /// Reads messages written since the previous read, advancing the reader's cursor.
    ///
    /// Messages that are no longer retained in the queue cannot be read.
    pub fn read(&mut self) -> impl Iterator<Item = &'a T> {
        let start = *self.cursor;
        let end = self.messages.message_count;
        *self.cursor = end;
        self.messages.iter_from_range(start, end)
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
