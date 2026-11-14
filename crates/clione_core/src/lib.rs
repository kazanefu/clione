//! A small, dependency-free ECS foundation for terminal game engines.
//!
//! A typical tick creates or updates entities through systems:
//!
//! ```
//! use clione_core::{Schedule, ScheduleStage, World};
//!
//! let mut world = World::new();
//! world.insert_resource(0_u32);
//! let mut schedule = Schedule::new();
//! schedule.add_system(ScheduleStage::Update, |world: &mut World| {
//!     *world.resource_mut::<u32>().unwrap() += 1;
//! });
//! schedule.run(&mut world);
//! assert_eq!(world.resource::<u32>(), Some(&1));
//! ```
#![warn(missing_docs)]

mod app;
mod commands;
mod entity;
mod events;
mod plugin;
mod query;
mod schedule;
mod state;
mod storage;
mod system_param;
mod world;

pub use app::{App, AppExit, init_state};
pub use commands::Commands;
pub use entity::Entity;
pub use events::Events;
pub use plugin::Plugin;
pub use query::{Query, Query2, QueryMut, QueryMutIter, QueryMutSpec, QuerySpec};
pub use schedule::{
    IntoScheduleTarget, OnEnter, OnExit, ReadOnlySystem, Schedule, ScheduleError, ScheduleStage,
    ScheduleTarget, System, SystemId,
};
pub use state::State;
pub use system_param::SystemContext;
pub use world::World;

#[cfg(test)]
mod tests;
