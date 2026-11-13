pub struct Events<T> {
    items: Vec<T>,
}

impl<T> Default for Events<T> {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}

impl<T> Events<T> {
    pub fn send(&mut self, event: T) {
        self.items.push(event);
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter()
    }

    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.items.drain(..)
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}
