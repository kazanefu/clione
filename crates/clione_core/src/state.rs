/// A world resource representing the current application state.
#[derive(Debug)]
pub struct State<S> {
    current: S,
    changed_from: Option<S>,
    pending: bool,
}

impl<S> State<S> {
    /// Creates a state resource with an initial value.
    pub fn new(initial: S) -> Self {
        Self {
            current: initial,
            changed_from: None,
            pending: true,
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
        self.changed_from = Some(std::mem::replace(&mut self.current, next));
        self.pending = true;
        true
    }

    pub(crate) fn take_transition(&mut self) -> Option<Option<S>> {
        self.pending.then(|| {
            self.pending = false;
            self.changed_from.take()
        })
    }
}
