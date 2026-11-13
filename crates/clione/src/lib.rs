//! Public facade for the Clione terminal game engine.

pub use clione_core;
pub use clione_core::{
    Commands, Entity, Events, Query, Query2, QueryMut, QueryMutIter, QueryMutSpec, QuerySpec,
    ReadOnlySystem, Schedule, ScheduleError, ScheduleStage, State, System, SystemId, World,
};
