use std::{any::TypeId, marker::PhantomData};

use crate::{
    Commands, Entity, MessageReader, MessageWriter, World,
    events::Messages,
    query::{Query, QueryMut, QueryMutSpec, QuerySpec},
};

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum AccessKey {
    Component(TypeId),
    Resource(TypeId),
}

/// A collection of system parameters borrowed from a world.
///
/// The context allows multiple queries and resources to coexist without
/// holding overlapping Rust borrows of `World`. Access conflicts are checked
/// once when a parameter is requested.
pub struct SystemContext<'world> {
    world: *mut World,
    system_id: Option<usize>,
    accesses: Vec<(AccessKey, bool)>,
    marker: PhantomData<&'world mut World>,
}

impl<'world> SystemContext<'world> {
    pub(crate) fn new(world: &'world mut World, system_id: Option<usize>) -> Self {
        Self {
            world,
            system_id,
            accesses: Vec::new(),
            marker: PhantomData,
        }
    }

    /// Creates a mutable or immutable component query.
    pub fn query<Q>(&mut self) -> Query<'world, Q>
    where
        Q: QuerySpec<'world>,
    {
        let types = Q::component_types();
        self.register_components(types, false);
        let entities = unsafe { (&*self.world).entities_with_types(&Q::component_types()) };
        Query::from_raw(self.world as *const World, entities)
    }

    /// Creates a query that may mutably access its components.
    pub fn query_mut<Q>(&mut self) -> QueryMut<'world, Q>
    where
        Q: QueryMutSpec<'world>,
    {
        let types = Q::component_types();
        self.register_components(types, true);
        let entities = unsafe { (&*self.world).entities_with_types(&Q::component_types()) };
        QueryMut::from_raw(self.world, entities)
    }

    /// Returns an immutable resource reference.
    pub fn resource<T: Send + Sync + 'static>(&mut self) -> Option<&'world T> {
        self.register_resources(vec![TypeId::of::<T>()], false);
        unsafe {
            (&*self.world)
                .resource::<T>()
                .map(|resource| &*(resource as *const T))
        }
    }

    /// Returns a mutable resource reference.
    pub fn resource_mut<T: Send + Sync + 'static>(&mut self) -> Option<&'world mut T> {
        self.register_resources(vec![TypeId::of::<T>()], true);
        unsafe {
            (&mut *self.world)
                .resource_mut::<T>()
                .map(|resource| &mut *(resource as *mut T))
        }
    }

    /// Returns a reader for messages that have already been registered.
    pub fn message_reader<T: Send + Sync + 'static>(&mut self) -> Option<MessageReader<'world, T>> {
        self.register_resources(vec![TypeId::of::<Messages<T>>()], false);
        unsafe {
            let messages = (&*self.world).messages::<T>()?;
            let system_id = self.system_id?;
            let cursor = (&mut *self.world)
                .message_cursors_mut()
                .entry((system_id, TypeId::of::<T>()))
                .or_insert(0);
            Some(MessageReader::from_state(messages, cursor))
        }
    }

    /// Returns a writer for a message queue, registering it when necessary.
    pub fn message_writer<T: Send + Sync + 'static>(&mut self) -> MessageWriter<'world, T> {
        self.register_resources(vec![TypeId::of::<Messages<T>>()], true);
        unsafe {
            if (&*self.world).messages::<T>().is_none() {
                (&mut *self.world).add_messages::<T>();
            }
            let messages = (&mut *self.world)
                .messages_mut::<T>()
                .expect("message resource was initialized above");
            MessageWriter::new(messages)
        }
    }

    /// Returns the world's deferred command buffer.
    pub fn commands(&mut self) -> &'world mut Commands {
        unsafe { &mut *(*self.world).commands_raw() }
    }

    /// Reserves an entity through the deferred command buffer.
    pub fn spawn(&mut self) -> Entity {
        unsafe { (&mut *self.world).spawn_deferred() }
    }

    fn register_components(&mut self, types: Vec<TypeId>, mutable: bool) {
        self.register(types.into_iter().map(AccessKey::Component), mutable);
    }

    fn register_resources(&mut self, types: Vec<TypeId>, mutable: bool) {
        self.register(types.into_iter().map(AccessKey::Resource), mutable);
    }

    fn register(&mut self, types: impl Iterator<Item = AccessKey>, mutable: bool) {
        for type_id in types {
            if let Some((_, previously_mutable)) =
                self.accesses.iter().find(|(key, _)| *key == type_id)
            {
                assert!(
                    !*previously_mutable && !mutable,
                    "system parameters have conflicting access to the same type"
                );
            } else {
                self.accesses.push((type_id, mutable));
            }
        }
    }
}
