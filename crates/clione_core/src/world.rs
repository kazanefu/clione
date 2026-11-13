use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use crate::{
    Entity,
    events::Events,
    storage::{ComponentStore, TypedStorage, component_type},
};

#[derive(Default)]
pub struct World {
    generations: Vec<u32>,
    free_entities: Vec<u32>,
    components: HashMap<TypeId, Box<dyn ComponentStore>>,
    resources: HashMap<TypeId, Box<dyn Any>>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(&mut self) -> Entity {
        let index = self.free_entities.pop().unwrap_or_else(|| {
            let index = self.generations.len() as u32;
            self.generations.push(0);
            index
        });

        Entity {
            index,
            generation: self.generations[index as usize],
        }
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }

        self.generations[entity.index as usize] =
            self.generations[entity.index as usize].wrapping_add(1);
        self.free_entities.push(entity.index);
        for storage in self.components.values_mut() {
            storage.remove(entity);
        }
        true
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        self.generations
            .get(entity.index as usize)
            .is_some_and(|generation| *generation == entity.generation)
    }

    pub fn insert<T: 'static>(&mut self, entity: Entity, component: T) -> bool {
        if !self.is_alive(entity) {
            return false;
        }

        let storage = self
            .components
            .entry(component_type::<T>())
            .or_insert_with(|| {
                Box::new(TypedStorage::<T> {
                    values: HashMap::new(),
                })
            });
        storage
            .as_any_mut()
            .downcast_mut::<TypedStorage<T>>()
            .expect("component storage type matches its TypeId")
            .values
            .insert(entity, component);
        true
    }

    pub fn remove<T: 'static>(&mut self, entity: Entity) -> Option<T> {
        self.components
            .get_mut(&component_type::<T>())
            .and_then(|storage| {
                storage
                    .as_any_mut()
                    .downcast_mut::<TypedStorage<T>>()
                    .expect("component storage type matches its TypeId")
                    .values
                    .remove(&entity)
            })
    }

    pub fn get<T: 'static>(&self, entity: Entity) -> Option<&T> {
        self.components
            .get(&component_type::<T>())
            .and_then(|storage| storage.as_any().downcast_ref::<TypedStorage<T>>())
            .and_then(|storage| storage.values.get(&entity))
    }

    pub fn get_mut<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        self.components
            .get_mut(&component_type::<T>())
            .and_then(|storage| storage.as_any_mut().downcast_mut::<TypedStorage<T>>())
            .and_then(|storage| storage.values.get_mut(&entity))
    }

    pub fn entities_with<T: 'static>(&self) -> Vec<Entity> {
        self.components
            .get(&component_type::<T>())
            .map_or_else(Vec::new, |storage| {
                let mut entities: Vec<_> = storage
                    .entities()
                    .into_iter()
                    .filter(|entity| self.is_alive(*entity))
                    .collect();
                entities.sort_by_key(|entity| (entity.index, entity.generation));
                entities
            })
    }

    pub fn insert_resource<T: 'static>(&mut self, resource: T) {
        self.resources.insert(TypeId::of::<T>(), Box::new(resource));
    }

    pub fn resource<T: 'static>(&self) -> Option<&T> {
        self.resources
            .get(&TypeId::of::<T>())
            .and_then(|resource| resource.downcast_ref::<T>())
    }

    pub fn resource_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.resources
            .get_mut(&TypeId::of::<T>())
            .and_then(|resource| resource.downcast_mut::<T>())
    }

    pub fn add_events<T: 'static>(&mut self) {
        if self.resource::<Events<T>>().is_none() {
            self.insert_resource(Events::<T>::default());
        }
    }

    pub fn add_event<T: 'static>(&mut self, event: T) {
        self.send_event(event);
    }

    pub fn send_event<T: 'static>(&mut self, event: T) {
        self.add_events::<T>();
        self.events_mut::<T>()
            .expect("event resource was initialized above")
            .send(event);
    }

    pub fn events<T: 'static>(&self) -> Option<&Events<T>> {
        self.resource::<Events<T>>()
    }

    pub fn events_mut<T: 'static>(&mut self) -> Option<&mut Events<T>> {
        self.resource_mut::<Events<T>>()
    }
}
