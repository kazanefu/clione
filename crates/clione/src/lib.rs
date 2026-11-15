//! Public facade for the Clione terminal game engine.

pub use clione_core;
pub use clione_core::{
    Added, App, AppExit, Changed, Commands, Entity, Events, FixedTime, IntoScheduleTarget,
    MessageReader, MessageWriter, Messages, OnEnter, OnEvent, OnExit, OnMessage, ParamSystem,
    Plugin, Query, Query2, QueryMut, QueryMutIter, QueryMutSpec, QuerySpec, ReadOnlySystem,
    Schedule, ScheduleError, ScheduleStage, ScheduleTarget, State, System, SystemContext, SystemId,
    Time, VirtualTime, With, Without, World, WorldError, init_state,
};

/// Commonly used Clione types.
pub mod prelude {
    pub use crate::{
        Added, App, AppExit, Changed, Commands, Entity, Events, FixedTime, IntoScheduleTarget,
        MessageReader, MessageWriter, Messages, OnEnter, OnEvent, OnExit, OnMessage, ParamSystem,
        Plugin, Query, Query2, QueryMut, QueryMutIter, QueryMutSpec, QuerySpec, ReadOnlySystem,
        Schedule, ScheduleError, ScheduleStage, State, System, SystemContext, SystemId, Time,
        VirtualTime, With, Without, World, WorldError, init_state,
    };
}
