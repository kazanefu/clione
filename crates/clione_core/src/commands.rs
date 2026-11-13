use crate::{Entity, World};

type Command = Box<dyn FnOnce(&mut World) + Send>;

/// Deferred structural changes to apply after a system has finished.
#[derive(Default)]
pub struct Commands {
    queued: Vec<Command>,
}

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

    /// Queues insertion or replacement of a component.
    pub fn insert<T: 'static + Send + Sync>(&mut self, entity: Entity, component: T) {
        self.queued.push(Box::new(move |world| {
            world.insert(entity, component);
        }));
    }

    /// Applies all queued structural changes in insertion order.
    pub fn apply(self, world: &mut World) {
        for command in self.queued {
            command(world);
        }
    }
}
