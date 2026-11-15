use crate::{Entity, World};

type Command = Box<dyn FnOnce(&mut World) + Send>;

/// Deferred structural changes to apply after a system has finished.
#[derive(Default)]
pub struct Commands {
    queued: Vec<Command>,
}

// Commands are only mutated through `&mut World`; queued closures are never
// accessed concurrently by read-only systems.
unsafe impl Sync for Commands {}

impl Commands {
    /// Creates an empty command buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues an entity despawn.
    pub fn despawn(&mut self, entity: Entity) {
        self.queued.push(Box::new(move |world| {
            world.despawn(entity);
        }));
    }

    pub(crate) fn spawn_reserved(&mut self, entity: Entity) {
        self.queued.push(Box::new(move |world| {
            world.activate_reserved(entity);
        }));
    }

    /// Queues insertion or replacement of a component.
    pub fn insert<T: 'static + Send + Sync>(&mut self, entity: Entity, component: T) {
        self.queued.push(Box::new(move |world| {
            world.insert(entity, component);
        }));
    }

    /// Queues removal of a component.
    pub fn remove<T: 'static + Send + Sync>(&mut self, entity: Entity) {
        self.queued.push(Box::new(move |world| {
            world.remove::<T>(entity);
        }));
    }

    /// Applies all queued structural changes in insertion order.
    pub fn apply(&mut self, world: &mut World) {
        let queued = std::mem::take(&mut self.queued);
        for command in queued {
            command(world);
        }
    }

    /// Returns whether commands are waiting to be applied.
    pub fn is_empty(&self) -> bool {
        self.queued.is_empty()
    }
}
