use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use crate::Entity;

pub(crate) trait ComponentStore: Any {
    fn remove(&mut self, entity: Entity);
    fn entities(&self) -> Vec<Entity>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub(crate) struct TypedStorage<T> {
    pub(crate) values: HashMap<Entity, T>,
}

impl<T: 'static> ComponentStore for TypedStorage<T> {
    fn remove(&mut self, entity: Entity) {
        self.values.remove(&entity);
    }

    fn entities(&self) -> Vec<Entity> {
        self.values.keys().copied().collect()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub(crate) fn component_type<T: 'static>() -> TypeId {
    TypeId::of::<T>()
}
