//! Public facade for the Clione terminal game engine.

pub use clione_core;
pub use clione_core::{
    Commands, Entity, Events, Query2, ReadOnlySystem, Schedule, ScheduleError, ScheduleStage,
    System, SystemId, World,
};
