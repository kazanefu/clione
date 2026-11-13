use std::marker::PhantomData;

use crate::{Entity, World};

/// A read-only query over entities containing two component types.
pub struct Query2<'world, A, B> {
    world: &'world World,
    entities: Vec<Entity>,
    marker: PhantomData<fn() -> (A, B)>,
}

impl<'world, A: 'static + Send + Sync, B: 'static + Send + Sync> Query2<'world, A, B> {
    pub(crate) fn new(world: &'world World, entities: Vec<Entity>) -> Self {
        Self {
            world,
            entities,
            marker: PhantomData,
        }
    }

    /// Returns all matching entities and both component references.
    pub fn iter(&self) -> impl Iterator<Item = (Entity, &A, &B)> {
        self.entities.iter().filter_map(|&entity| {
            Some((
                entity,
                self.world.get::<A>(entity)?,
                self.world.get::<B>(entity)?,
            ))
        })
    }

    /// Returns the matching components for one entity.
    pub fn get(&self, entity: Entity) -> Option<(&A, &B)> {
        Some((self.world.get(entity)?, self.world.get(entity)?))
    }

    /// Returns the number of entities matched when the query was created.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Returns whether the query matched no entities.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}
