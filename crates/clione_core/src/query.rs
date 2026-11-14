use std::{any::TypeId, marker::PhantomData};

use crate::{Entity, World};

/// Describes an immutable component query, such as `(&Position, &Velocity)`.
pub trait QuerySpec<'world> {
    /// The item yielded for one matching entity.
    type Item;

    /// Returns component types required by the query.
    fn component_types() -> Vec<TypeId>;

    /// # Safety
    ///
    /// `world` must be a valid reference for `'world`.
    unsafe fn fetch(world: &'world World, entity: Entity) -> Option<Self::Item>;
}

/// Describes a query that may contain immutable and mutable component references.
pub trait QueryMutSpec<'world> {
    /// The item yielded for one matching entity.
    type Item;

    /// Returns component types required by the query.
    fn component_types() -> Vec<TypeId>;

    /// # Safety
    ///
    /// `world` must be uniquely borrowed for `'world` and the query must not
    /// request the same component type twice.
    unsafe fn fetch(world: &'world mut World, entity: Entity) -> Option<Self::Item>;
}

impl<'world, T: 'static + Send + Sync> QuerySpec<'world> for &'world T {
    type Item = &'world T;

    fn component_types() -> Vec<TypeId> {
        vec![TypeId::of::<T>()]
    }

    unsafe fn fetch(world: &'world World, entity: Entity) -> Option<Self::Item> {
        world.get(entity)
    }
}

impl<'world, T: 'static + Send + Sync> QueryMutSpec<'world> for &'world T {
    type Item = &'world T;

    fn component_types() -> Vec<TypeId> {
        vec![TypeId::of::<T>()]
    }

    unsafe fn fetch(world: &'world mut World, entity: Entity) -> Option<Self::Item> {
        world.get(entity)
    }
}

impl<'world, T: 'static + Send + Sync> QueryMutSpec<'world> for &'world mut T {
    type Item = &'world mut T;

    fn component_types() -> Vec<TypeId> {
        vec![TypeId::of::<T>()]
    }

    unsafe fn fetch(world: &'world mut World, entity: Entity) -> Option<Self::Item> {
        world.get_mut(entity)
    }
}

macro_rules! impl_query_tuple {
    ($(($($name:ident),+)),+ $(,)?) => {
        $(
            impl<'world, $($name: QuerySpec<'world>),+> QuerySpec<'world>
                for ($($name,)+)
            {
                type Item = ($($name::Item,)+);

                fn component_types() -> Vec<TypeId> {
                    let mut types = Vec::new();
                    $(types.extend($name::component_types());)+
                    types
                }

                unsafe fn fetch(
                    world: &'world World,
                    entity: Entity,
                ) -> Option<Self::Item> {
                    Some(($(unsafe { $name::fetch(world, entity) }?,)+))
                }
            }

            impl<'world, $($name: QueryMutSpec<'world>),+> QueryMutSpec<'world>
                for ($($name,)+)
            {
                type Item = ($($name::Item,)+);

                fn component_types() -> Vec<TypeId> {
                    let mut types = Vec::new();
                    $(types.extend($name::component_types());)+
                    types
                }

                unsafe fn fetch(
                    world: &'world mut World,
                    entity: Entity,
                ) -> Option<Self::Item> {
                    let world = world as *mut World;
                    Some((
                        $(unsafe { $name::fetch(&mut *world, entity) }?,)+
                    ))
                }
            }
        )+
    };
}

impl_query_tuple! {
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F),
    (A, B, C, D, E, F, G),
    (A, B, C, D, E, F, G, H),
}

/// A read-only query over any supported tuple of component references.
pub struct Query<'world, Q: QuerySpec<'world>> {
    world: *const World,
    entities: Vec<Entity>,
    marker: PhantomData<&'world Q>,
}

impl<'world, Q: QuerySpec<'world>> Query<'world, Q> {
    pub(crate) fn new(world: &'world World, entities: Vec<Entity>) -> Self {
        Self {
            world: world as *const World,
            entities,
            marker: PhantomData,
        }
    }

    pub(crate) fn from_raw(world: *const World, entities: Vec<Entity>) -> Self {
        Self {
            world,
            entities,
            marker: PhantomData,
        }
    }

    /// Returns all matching entities and query items.
    pub fn iter(&self) -> impl Iterator<Item = (Entity, Q::Item)> + '_ {
        self.entities
            .iter()
            .filter_map(|&entity| Some((entity, unsafe { Q::fetch(&*self.world, entity) }?)))
    }

    /// Returns the query item for one entity.
    pub fn get(&self, entity: Entity) -> Option<Q::Item> {
        unsafe { Q::fetch(&*self.world, entity) }
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

/// A query that can yield immutable and mutable component references.
pub struct QueryMut<'world, Q: QueryMutSpec<'world>> {
    world: *mut World,
    entities: Vec<Entity>,
    marker: PhantomData<&'world mut Q>,
}

impl<'world, Q: QueryMutSpec<'world>> QueryMut<'world, Q> {
    pub(crate) fn new(world: &'world mut World, entities: Vec<Entity>) -> Self {
        assert_unique_types(&Q::component_types());
        Self {
            world: world as *mut World,
            entities,
            marker: PhantomData,
        }
    }

    pub(crate) fn from_raw(world: *mut World, entities: Vec<Entity>) -> Self {
        assert_unique_types(&Q::component_types());
        Self {
            world,
            entities,
            marker: PhantomData,
        }
    }

    /// Returns a mutable iterator over matching entities and query items.
    pub fn iter_mut(&mut self) -> QueryMutIter<'_, 'world, Q> {
        QueryMutIter {
            world: self.world,
            entities: self.entities.iter().copied(),
            marker: PhantomData,
        }
    }

    /// Returns the mutable query item for one entity.
    pub fn get_mut(&mut self, entity: Entity) -> Option<Q::Item> {
        unsafe { fetch_mut_at::<Q>(self.world, entity) }
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

/// Iterator returned by [`QueryMut::iter_mut`].
pub struct QueryMutIter<'query, 'world, Q: QueryMutSpec<'world>> {
    world: *mut World,
    entities: std::iter::Copied<std::slice::Iter<'query, Entity>>,
    marker: PhantomData<&'world mut Q>,
}

impl<'query, 'world, Q: QueryMutSpec<'world>> Iterator for QueryMutIter<'query, 'world, Q> {
    type Item = (Entity, Q::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let entity = self.entities.next()?;
        Some((entity, unsafe { fetch_mut_at::<Q>(self.world, entity) }?))
    }
}

unsafe fn fetch_mut_at<'world, Q: QueryMutSpec<'world>>(
    world: *mut World,
    entity: Entity,
) -> Option<Q::Item> {
    unsafe { Q::fetch(&mut *world, entity) }
}

fn assert_unique_types(types: &[TypeId]) {
    for (index, type_id) in types.iter().enumerate() {
        assert!(
            !types[index + 1..].contains(type_id),
            "a query cannot borrow the same component type more than once"
        );
    }
}

/// A compatibility alias for the original two-component query API.
pub type Query2<'world, A, B> = Query<'world, (&'world A, &'world B)>;
