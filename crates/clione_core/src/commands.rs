use crate::{Entity, World};

type Command = Box<dyn FnOnce(&mut World)>;

#[derive(Default)]
pub struct Commands {
    queued: Vec<Command>,
}

impl Commands {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn despawn(&mut self, entity: Entity) {
        self.queued.push(Box::new(move |world| {
            world.despawn(entity);
        }));
    }

    pub fn insert<T: 'static>(&mut self, entity: Entity, component: T) {
        self.queued.push(Box::new(move |world| {
            world.insert(entity, component);
        }));
    }

    pub fn apply(self, world: &mut World) {
        for command in self.queued {
            command(world);
        }
    }
}
