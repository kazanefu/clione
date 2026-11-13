/// A generational handle to an entity in a [`World`](crate::World).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Entity {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

impl Entity {
    /// Returns the stable slot index of this entity.
    pub fn index(self) -> u32 {
        self.index
    }

    /// Returns the generation used to invalidate stale handles.
    pub fn generation(self) -> u32 {
        self.generation
    }
}
