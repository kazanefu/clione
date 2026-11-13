/// A world resource representing the current application state.
#[derive(Debug)]
pub struct State<S> {
    current: S,
    changed: bool,
}

impl<S> State<S> {
    /// Creates a state resource with an initial value.
    pub fn new(initial: S) -> Self {
        Self {
            current: initial,
            changed: true,
        }
    }

    /// Returns the current state value.
    pub fn get(&self) -> &S {
        &self.current
    }

    /// Changes the state and marks an enter transition when the value differs.
    pub fn set(&mut self, next: S) -> bool
    where
        S: PartialEq,
    {
        if self.current == next {
            return false;
        }
        self.current = next;
        self.changed = true;
        true
    }

    pub(crate) fn take_changed(&mut self) -> bool {
        let changed = self.changed;
        self.changed = false;
        changed
    }
}
