use std::{any::TypeId, marker::PhantomData};

use crate::{
    World,
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
    accesses: Vec<(AccessKey, bool)>,
    marker: PhantomData<&'world mut World>,
}

impl<'world> SystemContext<'world> {
    pub(crate) fn new(world: &'world mut World) -> Self {
        Self {
            world,
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
