use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use crate::Entity;

pub(crate) trait Column: Any + Send + Sync {
    fn empty(&self) -> Box<dyn Column>;
    fn remove_value(&mut self, row: usize) -> Box<dyn Any + Send + Sync>;
    fn push_value(&mut self, value: Box<dyn Any + Send + Sync>);
    fn get_ptr(&self, row: usize) -> *const ();
    fn get_mut_ptr(&mut self, row: usize) -> *mut ();
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub(crate) struct TypedColumn<T> {
    pub(crate) values: Vec<T>,
}

impl<T: 'static + Send + Sync> Column for TypedColumn<T> {
    fn empty(&self) -> Box<dyn Column> {
        Box::new(Self { values: Vec::new() })
    }

    fn remove_value(&mut self, row: usize) -> Box<dyn Any + Send + Sync> {
        Box::new(self.values.remove(row))
    }

    fn push_value(&mut self, value: Box<dyn Any + Send + Sync>) {
        self.values
            .push(*value.downcast::<T>().expect("column type matches TypeId"));
    }

    fn get_ptr(&self, row: usize) -> *const () {
        &self.values[row] as *const T as *const ()
    }

    fn get_mut_ptr(&mut self, row: usize) -> *mut () {
        &mut self.values[row] as *mut T as *mut ()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub(crate) struct Archetype {
    pub(crate) types: Vec<TypeId>,
    pub(crate) entities: Vec<Entity>,
    pub(crate) columns: HashMap<TypeId, Box<dyn Column>>,
}

impl Archetype {
    pub(crate) fn new(types: Vec<TypeId>, columns: HashMap<TypeId, Box<dyn Column>>) -> Self {
        Self {
            types,
            entities: Vec::new(),
            columns,
        }
    }

    pub(crate) fn add_entity(
        &mut self,
        entity: Entity,
        values: impl IntoIterator<Item = (TypeId, Box<dyn Any + Send + Sync>)>,
    ) {
        self.entities.push(entity);
        for (type_id, value) in values {
            self.columns
                .get_mut(&type_id)
                .expect("archetype has a column for every component")
                .push_value(value);
        }
    }

    pub(crate) fn remove_entity(
        &mut self,
        entity: Entity,
    ) -> Option<Vec<(TypeId, Box<dyn Any + Send + Sync>)>> {
        let row = self
            .entities
            .iter()
            .position(|candidate| *candidate == entity)?;

        let values = self
            .types
            .iter()
            .map(|type_id| {
                (
                    *type_id,
                    self.columns
                        .get_mut(type_id)
                        .expect("archetype has a column for every component")
                        .remove_value(row),
                )
            })
            .collect();
        self.entities.swap_remove(row);
        Some(values)
    }
}

pub(crate) fn component_type<T: 'static>() -> TypeId {
    TypeId::of::<T>()
}

pub(crate) fn typed_column<T: 'static + Send + Sync>() -> Box<dyn Column> {
    Box::new(TypedColumn::<T> { values: Vec::new() })
}
