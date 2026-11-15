use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet, hash_map::DefaultHasher},
    fmt,
    hash::{Hash, Hasher},
};

use crate::{
    Entity,
    commands::Commands,
    events::Events,
    query::{Query, Query2, QueryMut, QueryMutSpec, QuerySpec},
    storage::{Archetype, Column, component_type, typed_column},
    system_param::SystemContext,
};

type ComponentValue = (TypeId, Box<dyn Any + Send + Sync>);
type ColumnTemplates = HashMap<TypeId, Box<dyn Column>>;

/// Errors returned by fallible World operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldError {
    /// The entity handle is stale or does not exist in this world.
    InvalidEntity(Entity),
    /// The entity does not contain the requested component.
    MissingComponent {
        /// Entity on which the operation was attempted.
        entity: Entity,
        /// Type identifier of the missing component.
        component: TypeId,
    },
}

impl fmt::Display for WorldError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEntity(entity) => write!(formatter, "invalid entity: {entity:?}"),
            Self::MissingComponent { entity, component } => {
                write!(
                    formatter,
                    "entity {entity:?} has no component {component:?}"
                )
            }
        }
    }
}

impl std::error::Error for WorldError {}

/// The ECS container for entities, components, resources, events, and archetypes.
#[derive(Default)]
pub struct World {
    generations: Vec<u32>,
    free_entities: Vec<u32>,
    reserved_entities: HashSet<Entity>,
    archetypes: HashMap<Vec<TypeId>, Archetype>,
    locations: HashMap<Entity, (Vec<TypeId>, usize)>,
    resources: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
    changed_resources: HashSet<TypeId>,
    changed_components: HashSet<TypeId>,
    component_changes: HashMap<(Entity, TypeId), u64>,
    added_components: HashSet<(Entity, TypeId)>,
    next_component_generation: u64,
    event_clearers: Vec<fn(&mut World)>,
    message_cursors: HashMap<(usize, TypeId), usize>,
    commands: Commands,
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
        SystemContext::new(self, None)
    }

    pub(crate) fn system_context_for(&mut self, system_id: usize) -> SystemContext<'_> {
        SystemContext::new(self, Some(system_id))
    }

    pub(crate) fn message_cursors_mut(&mut self) -> &mut HashMap<(usize, TypeId), usize> {
        &mut self.message_cursors
    }

    /// Allocates a new entity.
    pub fn spawn(&mut self) -> Entity {
        let entity = self.allocate_entity();
        self.activate_reserved(entity);
        entity
    }

    /// Reserves an entity and queues its activation through the world's commands.
    pub fn spawn_deferred(&mut self) -> Entity {
        let entity = self.allocate_entity();
        self.reserved_entities.insert(entity);
        self.commands.spawn_reserved(entity);
        entity
    }

    fn allocate_entity(&mut self) -> Entity {
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

    pub(crate) fn activate_reserved(&mut self, entity: Entity) {
        self.reserved_entities.remove(&entity);
        let archetype = self.archetypes.get_mut(&Vec::new()).unwrap();
        archetype.add_entity(entity, []);
        self.locations
            .insert(entity, (Vec::new(), archetype.entities.len() - 1));
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

    /// Despawns an entity or returns an explicit error.
    pub fn try_despawn(&mut self, entity: Entity) -> Result<(), WorldError> {
        self.despawn(entity)
            .then_some(())
            .ok_or(WorldError::InvalidEntity(entity))
    }

    /// Returns whether an entity handle points to a live entity.
    pub fn is_alive(&self, entity: Entity) -> bool {
        !self.reserved_entities.contains(&entity)
            && self
                .generations
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
            self.mark_component_changed(entity, type_id, false);
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

        self.mark_component_changed(entity, type_id, true);
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

    /// Inserts or replaces a component, returning an explicit error for a stale entity.
    pub fn try_insert<T: 'static + Send + Sync>(
        &mut self,
        entity: Entity,
        component: T,
    ) -> Result<(), WorldError> {
        self.insert(entity, component)
            .then_some(())
            .ok_or(WorldError::InvalidEntity(entity))
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
        self.mark_component_changed(entity, type_id, false);
        self.added_components.remove(&(entity, type_id));
        let (mut values, mut columns) = self.remove_from_current_archetype(entity);
        let position = values.iter().position(|(id, _)| *id == type_id)?;
        let (_, value) = values.swap_remove(position);
        columns.remove(&type_id);
        let types = values.iter().map(|(id, _)| *id).collect();
        self.move_entity(entity, types, values, columns);
        Some(*value.downcast::<T>().expect("column type matches TypeId"))
    }

    /// Removes a component or returns an explicit missing-component error.
    pub fn try_remove<T: 'static + Send + Sync>(
        &mut self,
        entity: Entity,
    ) -> Result<T, WorldError> {
        if !self.is_alive(entity) {
            return Err(WorldError::InvalidEntity(entity));
        }
        self.remove::<T>(entity)
            .ok_or(WorldError::MissingComponent {
                entity,
                component: TypeId::of::<T>(),
            })
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
        let type_id = component_type::<T>();
        let has_component = self.locations.get(&entity).is_some_and(|(signature, _)| {
            self.archetypes
                .get(signature)
                .is_some_and(|archetype| archetype.columns.contains_key(&type_id))
        });
        if !has_component {
            return None;
        }
        self.mark_component_changed(entity, type_id, false);
        self.get_mut_untracked(entity)
    }

    pub(crate) fn get_mut_untracked<T: 'static + Send + Sync>(
        &mut self,
        entity: Entity,
    ) -> Option<&mut T> {
        let (signature, row) = self.locations.get(&entity)?.clone();
        let type_id = component_type::<T>();
        let column = self
            .archetypes
            .get_mut(&signature)?
            .columns
            .get_mut(&type_id)?;
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
        self.changed_resources.insert(TypeId::of::<T>());
    }

    /// Returns a world-global resource.
    pub fn resource<T: 'static + Send + Sync>(&self) -> Option<&T> {
        self.resources
            .get(&TypeId::of::<T>())
            .and_then(|resource| resource.downcast_ref::<T>())
    }

    /// Returns a mutable world-global resource.
    pub fn resource_mut<T: 'static + Send + Sync>(&mut self) -> Option<&mut T> {
        self.changed_resources.insert(TypeId::of::<T>());
        self.resources
            .get_mut(&TypeId::of::<T>())
            .and_then(|resource| resource.downcast_mut::<T>())
    }

    /// Returns whether a resource was inserted or mutably accessed.
    pub fn is_resource_changed<T: 'static + Send + Sync>(&self) -> bool {
        self.changed_resources.contains(&TypeId::of::<T>())
    }

    /// Clears the change flag for one resource.
    pub fn clear_resource_changed<T: 'static + Send + Sync>(&mut self) {
        self.changed_resources.remove(&TypeId::of::<T>());
    }

    /// Clears all resource change flags for the current tick.
    pub fn clear_resource_changes(&mut self) {
        self.changed_resources.clear();
    }

    /// Returns whether a component type changed during the current tick.
    pub fn is_component_changed<T: 'static + Send + Sync>(&self) -> bool {
        self.changed_components.contains(&TypeId::of::<T>())
    }

    /// Returns whether an entity received this component since the last reset.
    pub fn is_component_added<T: 'static + Send + Sync>(&self, entity: Entity) -> bool {
        self.added_components.contains(&(entity, TypeId::of::<T>()))
    }

    /// Returns whether an entity's component changed since the last reset.
    pub fn is_entity_component_changed<T: 'static + Send + Sync>(&self, entity: Entity) -> bool {
        self.component_changes
            .contains_key(&(entity, TypeId::of::<T>()))
    }

    /// Returns the latest change generation for an entity component.
    pub fn component_change_generation<T: 'static + Send + Sync>(
        &self,
        entity: Entity,
    ) -> Option<u64> {
        self.component_changes
            .get(&(entity, TypeId::of::<T>()))
            .copied()
    }

    /// Clears all component change flags for the current tick.
    pub fn clear_component_changes(&mut self) {
        self.changed_components.clear();
        self.component_changes.clear();
        self.added_components.clear();
    }

    /// Returns the deferred command buffer for this world.
    pub fn commands(&mut self) -> &mut Commands {
        &mut self.commands
    }

    pub(crate) fn commands_raw(&mut self) -> *mut Commands {
        &mut self.commands
    }

    /// Applies all deferred structural changes.
    pub fn flush_commands(&mut self) {
        let mut commands = std::mem::take(&mut self.commands);
        commands.apply(self);
        self.commands = commands;
    }

    /// Registers an event queue if it is not already present.
    pub fn add_events<T: 'static + Send + Sync>(&mut self) {
        if self.resource::<Events<T>>().is_none() {
            self.insert_resource(Events::<T>::default());
            self.event_clearers.push(clear_events::<T>);
        }
    }

    /// Registers a message queue if it is not already present.
    pub fn add_messages<T: 'static + Send + Sync>(&mut self) {
        self.add_events::<T>();
    }

    /// Clears all registered event queues.
    pub fn clear_events(&mut self) {
        let clearers = self.event_clearers.clone();
        for clear in clearers {
            clear(self);
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

    /// Sends a message, creating its queue when necessary.
    pub fn send_message<T: 'static + Send + Sync>(&mut self, message: T) {
        self.add_messages::<T>();
        self.messages_mut::<T>()
            .expect("message resource was initialized above")
            .write(message);
    }

    /// Returns an event queue.
    pub fn events<T: 'static + Send + Sync>(&self) -> Option<&Events<T>> {
        self.resource::<Events<T>>()
    }

    /// Returns a mutable event queue.
    pub fn events_mut<T: 'static + Send + Sync>(&mut self) -> Option<&mut Events<T>> {
        self.resource_mut::<Events<T>>()
    }

    /// Returns a message queue.
    pub fn messages<T: 'static + Send + Sync>(&self) -> Option<&Events<T>> {
        self.events::<T>()
    }

    /// Returns a mutable message queue.
    pub fn messages_mut<T: 'static + Send + Sync>(&mut self) -> Option<&mut Events<T>> {
        self.events_mut::<T>()
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

    fn mark_component_changed(&mut self, entity: Entity, type_id: TypeId, added: bool) {
        self.next_component_generation = self.next_component_generation.wrapping_add(1);
        self.component_changes
            .insert((entity, type_id), self.next_component_generation);
        if added {
            self.added_components.insert((entity, type_id));
        }
        self.changed_components.insert(type_id);
    }

    pub(crate) fn mark_component_type_changed(&mut self, entity: Entity, type_id: TypeId) {
        self.mark_component_changed(entity, type_id, false);
    }
}

fn clear_events<T: 'static + Send + Sync>(world: &mut World) {
    if let Some(events) = world.resources.get_mut(&TypeId::of::<Events<T>>()) {
        events
            .downcast_mut::<Events<T>>()
            .expect("event resource type matches")
            .clear();
    }
}

fn type_id_key(type_id: &TypeId) -> u64 {
    let mut hasher = DefaultHasher::new();
    type_id.hash(&mut hasher);
    hasher.finish()
}
