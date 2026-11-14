//! Public facade for the Clione terminal game engine.

pub use clione_core;
pub use clione_core::{
    App, AppExit, Commands, Entity, Events, IntoScheduleTarget, OnEnter, OnExit, Plugin, Query,
    Query2, QueryMut, QueryMutIter, QueryMutSpec, QuerySpec, ReadOnlySystem, Schedule,
    ScheduleError, ScheduleStage, ScheduleTarget, State, System, SystemId, World, init_state,
};

/// Commonly used Clione types.
pub mod prelude {
    pub use crate::{
        App, AppExit, Commands, Entity, Events, IntoScheduleTarget, OnEnter, OnExit, Plugin, Query,
        Query2, QueryMut, QueryMutIter, QueryMutSpec, QuerySpec, ReadOnlySystem, Schedule,
        ScheduleError, ScheduleStage, State, System, SystemId, World, init_state,
    };
}
