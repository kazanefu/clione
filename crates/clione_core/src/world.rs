use std::{
    any::{Any, TypeId},
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
};

use crate::{
    Entity,
    events::Events,
    query::{Query, Query2, QueryMut, QueryMutSpec, QuerySpec},
    storage::{Archetype, Column, component_type, typed_column},
    system_param::SystemContext,
};

type ComponentValue = (TypeId, Box<dyn Any + Send + Sync>);
type ColumnTemplates = HashMap<TypeId, Box<dyn Column>>;

/// The ECS container for entities, components, resources, events, and archetypes.
#[derive(Default)]
pub struct World {
    generations: Vec<u32>,
    free_entities: Vec<u32>,
    archetypes: HashMap<Vec<TypeId>, Archetype>,
    locations: HashMap<Entity, (Vec<TypeId>, usize)>,
    resources: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl World {
    /// Creates an empty world.
    pub fn new() -> Self {
        let mut world = Self::default();
        world.ensure_archetype(Vec::new());
        world
    }

    /// Creates a parameter context for obtaining multiple queries and resources.
    pub fn system_context(&mut self) -> SystemContext<'_> {
        SystemContext::new(self)
    }

    /// Allocates a new entity.
    pub fn spawn(&mut self) -> Entity {
        let index = self.free_entities.pop().unwrap_or_else(|| {
            let index = self.generations.len() as u32;
            self.generations.push(0);
            index
        });
        let entity = Entity {
            index,
            generation: self.generations[index as usize],
        };
        let archetype = self.archetypes.get_mut(&Vec::new()).unwrap();
        archetype.add_entity(entity, []);
        self.locations
            .insert(entity, (Vec::new(), archetype.entities.len() - 1));
        entity
    }

    /// Removes an entity and all of its components.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        self.remove_from_current_archetype(entity);
        self.generations[entity.index as usize] =
            self.generations[entity.index as usize].wrapping_add(1);
        self.free_entities.push(entity.index);
        true
    }

    /// Returns whether an entity handle points to a live entity.
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.generations
            .get(entity.index as usize)
            .is_some_and(|generation| *generation == entity.generation)
    }

    /// Inserts or replaces a component on an entity.
    pub fn insert<T: 'static + Send + Sync>(&mut self, entity: Entity, component: T) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        let type_id = component_type::<T>();
        let has_component = self
            .locations
            .get(&entity)
            .is_some_and(|(types, _)| types.contains(&type_id));
        if has_component {
            let (signature, row) = self.locations.get(&entity).unwrap().clone();
            let archetype = self.archetypes.get_mut(&signature).unwrap();
            let column = archetype.columns.get_mut(&type_id).unwrap();
            column
                .as_any_mut()
                .downcast_mut::<crate::storage::TypedColumn<T>>()
                .expect("column type matches TypeId")
                .values[row] = component;
            return true;
        }

        let (mut values, mut columns) = self.remove_from_current_archetype(entity);
        let mut types = values
            .iter()
            .map(|(type_id, _)| *type_id)
            .collect::<Vec<_>>();
        types.push(type_id);
        types.sort_by_key(type_id_key);
        columns.insert(type_id, typed_column::<T>());
        values.push((type_id, Box::new(component)));
        self.move_entity(entity, types, values, columns);
        true
    }

    /// Removes and returns a component from an entity.
    pub fn remove<T: 'static + Send + Sync>(&mut self, entity: Entity) -> Option<T> {
        if !self.is_alive(entity) {
            return None;
        }
        let type_id = component_type::<T>();
        if !self
            .locations
            .get(&entity)
            .is_some_and(|(types, _)| types.contains(&type_id))
        {
            return None;
        }
        let (mut values, mut columns) = self.remove_from_current_archetype(entity);
        let position = values.iter().position(|(id, _)| *id == type_id)?;
        let (_, value) = values.swap_remove(position);
        columns.remove(&type_id);
        let types = values.iter().map(|(id, _)| *id).collect();
        self.move_entity(entity, types, values, columns);
        Some(*value.downcast::<T>().expect("column type matches TypeId"))
    }

    /// Returns a component reference for an entity.
    pub fn get<T: 'static + Send + Sync>(&self, entity: Entity) -> Option<&T> {
        let (signature, row) = self.locations.get(&entity)?;
        let column = self
            .archetypes
            .get(signature)?
            .columns
            .get(&component_type::<T>())?;
        let pointer = column.get_ptr(*row) as *const T;
        // The row is owned by this world and remains valid for this shared borrow.
        Some(unsafe { &*pointer })
    }

    /// Returns a mutable component reference for an entity.
    pub fn get_mut<T: 'static + Send + Sync>(&mut self, entity: Entity) -> Option<&mut T> {
        let (signature, row) = self.locations.get(&entity)?.clone();
        let column = self
            .archetypes
            .get_mut(&signature)?
            .columns
            .get_mut(&component_type::<T>())?;
        let pointer = column.get_mut_ptr(row) as *mut T;
        // The mutable world borrow prevents structural changes while this reference exists.
        Some(unsafe { &mut *pointer })
    }

    /// Returns live entities that contain the component.
    pub fn entities_with<T: 'static + Send + Sync>(&self) -> Vec<Entity> {
        self.entities_with_types(&[component_type::<T>()])
    }

    /// Creates a read-only query matching entities with both components.
    pub fn query2<A: 'static + Send + Sync, B: 'static + Send + Sync>(&self) -> Query2<'_, A, B> {
        self.query::<(&A, &B)>()
    }

    /// Creates a read-only query from a tuple of component references.
    pub fn query<'world, Q>(&'world self) -> Query<'world, Q>
    where
        Q: QuerySpec<'world>,
    {
        Query::new(self, self.entities_with_types(&Q::component_types()))
    }

    /// Creates a query that can read or mutate components according to `Q`.
    pub fn query_mut<'world, Q>(&'world mut self) -> QueryMut<'world, Q>
    where
        Q: QueryMutSpec<'world>,
    {
        let entities = self.entities_with_types(&Q::component_types());
        QueryMut::new(self, entities)
    }

    pub(crate) fn entities_with_types(&self, required: &[TypeId]) -> Vec<Entity> {
        let mut entities = self
            .archetypes
            .values()
            .filter(|archetype| {
                required
                    .iter()
                    .all(|type_id| archetype.types.contains(type_id))
            })
            .flat_map(|archetype| archetype.entities.iter().copied())
            .collect::<Vec<_>>();
        entities.sort_by_key(|entity| (entity.index, entity.generation));
        entities
    }

    /// Inserts or replaces a world-global resource.
    pub fn insert_resource<T: 'static + Send + Sync>(&mut self, resource: T) {
        self.resources.insert(TypeId::of::<T>(), Box::new(resource));
    }

    /// Returns a world-global resource.
    pub fn resource<T: 'static + Send + Sync>(&self) -> Option<&T> {
        self.resources
            .get(&TypeId::of::<T>())
            .and_then(|resource| resource.downcast_ref::<T>())
    }

    /// Returns a mutable world-global resource.
    pub fn resource_mut<T: 'static + Send + Sync>(&mut self) -> Option<&mut T> {
        self.resources
            .get_mut(&TypeId::of::<T>())
            .and_then(|resource| resource.downcast_mut::<T>())
    }

    /// Registers an event queue if it is not already present.
    pub fn add_events<T: 'static + Send + Sync>(&mut self) {
        if self.resource::<Events<T>>().is_none() {
            self.insert_resource(Events::<T>::default());
        }
    }

    /// Registers an event queue and sends one event.
    pub fn add_event<T: 'static + Send + Sync>(&mut self, event: T) {
        self.send_event(event);
    }

    /// Sends an event, creating its queue when necessary.
    pub fn send_event<T: 'static + Send + Sync>(&mut self, event: T) {
        self.add_events::<T>();
        self.events_mut::<T>()
            .expect("event resource was initialized above")
            .send(event);
    }

    /// Returns an event queue.
    pub fn events<T: 'static + Send + Sync>(&self) -> Option<&Events<T>> {
        self.resource::<Events<T>>()
    }

    /// Returns a mutable event queue.
    pub fn events_mut<T: 'static + Send + Sync>(&mut self) -> Option<&mut Events<T>> {
        self.resource_mut::<Events<T>>()
    }

    fn ensure_archetype(&mut self, signature: Vec<TypeId>) {
        if self.archetypes.contains_key(&signature) {
            return;
        }
        self.archetypes
            .insert(signature.clone(), Archetype::new(signature, HashMap::new()));
    }

    fn remove_from_current_archetype(
        &mut self,
        entity: Entity,
    ) -> (Vec<ComponentValue>, ColumnTemplates) {
        let (signature, _) = self
            .locations
            .remove(&entity)
            .expect("live entity has a location");
        let archetype = self.archetypes.get_mut(&signature).unwrap();
        let columns = archetype
            .columns
            .iter()
            .map(|(type_id, column)| (*type_id, column.empty()))
            .collect();
        let values = archetype.remove_entity(entity).unwrap();
        self.refresh_locations(&signature);
        (values, columns)
    }

    fn move_entity(
        &mut self,
        entity: Entity,
        signature: Vec<TypeId>,
        values: Vec<ComponentValue>,
        columns: ColumnTemplates,
    ) {
        self.ensure_archetype_with_columns(&signature, columns);
        let archetype = self.archetypes.get_mut(&signature).unwrap();
        archetype.add_entity(entity, values);
        self.locations
            .insert(entity, (signature.clone(), archetype.entities.len() - 1));
    }

    fn ensure_archetype_with_columns(&mut self, signature: &[TypeId], columns: ColumnTemplates) {
        if self.archetypes.contains_key(signature) {
            return;
        }
        for type_id in signature {
            assert!(
                columns.contains_key(type_id),
                "every archetype component has a column"
            );
        }
        self.archetypes.insert(
            signature.to_vec(),
            Archetype::new(signature.to_vec(), columns),
        );
    }

    fn refresh_locations(&mut self, signature: &[TypeId]) {
        let archetype = self.archetypes.get(signature).unwrap();
        for (row, entity) in archetype.entities.iter().copied().enumerate() {
            self.locations.insert(entity, (signature.to_vec(), row));
        }
    }
}

fn type_id_key(type_id: &TypeId) -> u64 {
    let mut hasher = DefaultHasher::new();
    type_id.hash(&mut hasher);
    hasher.finish()
}
