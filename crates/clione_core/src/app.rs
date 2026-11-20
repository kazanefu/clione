use crate::{
    FixedTime, IntoScheduleTarget, Plugin, ReadOnlySystem, Schedule, ScheduleError, ScheduleStage,
    State, System, SystemId, Time, VirtualTime, World,
};
use std::time::Instant;

/// A resource that stops the application loop when requested.
#[derive(Debug, Default)]
pub struct AppExit {
    requested: bool,
}

impl AppExit {
    /// Requests that [`App::run`] stop after the current tick.
    pub fn request(&mut self) {
        self.requested = true;
    }

    /// Returns whether application shutdown was requested.
    pub fn is_requested(&self) -> bool {
        self.requested
    }
}

/// The application facade that owns the ECS world and schedule.
pub struct App {
    world: World,
    schedule: Schedule,
    last_update: Instant,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Creates an application with an empty world and schedule.
    pub fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(AppExit::default());
        world.insert_resource(Time::default());
        world.insert_resource(VirtualTime::default());
        world.insert_resource(FixedTime::default());
        Self {
            world,
            schedule: Schedule::new(),
            last_update: Instant::now(),
        }
    }

    /// Adds a system to a stage or state lifecycle target.
    pub fn add_systems<T, F>(&mut self, target: T, system: F) -> &mut Self
    where
        T: IntoScheduleTarget,
        F: System + 'static,
    {
        self.schedule.add_systems(target, system);
        self
    }

    /// Adds an ordered mutable system and returns its identifier.
    ///
    /// Use the identifier with [`App::add_dependency`] when explicit ordering
    /// is required between regular stage systems.
    pub fn add_system<F>(&mut self, stage: ScheduleStage, system: F) -> SystemId
    where
        F: System + 'static,
    {
        self.schedule.add_system(stage, system)
    }

    /// Adds a system that receives a [`SystemContext`] and returns its ID.
    pub fn add_param_system<F>(&mut self, stage: ScheduleStage, system: F) -> SystemId
    where
        F: crate::ParamSystem + 'static,
    {
        self.schedule.add_param_system(stage, system)
    }

    /// Makes one regular stage system run before another.
    pub fn add_dependency(
        &mut self,
        before: SystemId,
        after: SystemId,
    ) -> Result<&mut Self, ScheduleError> {
        self.schedule.add_dependency(before, after)?;
        Ok(self)
    }

    /// Builds and registers a plugin on this application.
    pub fn add_plugins<P>(&mut self, plugin: P) -> &mut Self
    where
        P: Plugin,
    {
        plugin.build(self);
        self
    }

    /// Adds a read-only system to a stage.
    pub fn add_read_only_system<F>(&mut self, stage: crate::ScheduleStage, system: F) -> &mut Self
    where
        F: ReadOnlySystem + 'static,
    {
        self.schedule.add_read_only_system(stage, system);
        self
    }

    /// Inserts a resource into the application world.
    pub fn insert_resource<T: Send + Sync + 'static>(&mut self, resource: T) -> &mut Self {
        self.world.insert_resource(resource);
        self
    }

    /// Returns an immutable reference to the application world.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Returns a mutable reference to the application world.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Returns an immutable reference to the application schedule.
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Returns a mutable reference to the application schedule.
    pub fn schedule_mut(&mut self) -> &mut Schedule {
        &mut self.schedule
    }

    /// Runs one complete application tick.
    pub fn update(&mut self) {
        let now = Instant::now();
        let delta = now.saturating_duration_since(self.last_update);
        self.last_update = now;
        self.update_with_delta(delta);
    }

    /// Runs one application tick with an explicit elapsed duration.
    pub fn update_with_delta(&mut self, delta: std::time::Duration) {
        self.world.resource_mut::<Time>().unwrap().advance(delta);
        self.world
            .resource_mut::<VirtualTime>()
            .unwrap()
            .advance(delta);
        let virtual_delta = self.world.resource::<VirtualTime>().unwrap().delta();
        self.world
            .resource_mut::<FixedTime>()
            .unwrap()
            .accumulate(virtual_delta);
        self.schedule.run_startup(&mut self.world);
        self.schedule.run_on_enter(&mut self.world);
        while self
            .world
            .resource_mut::<FixedTime>()
            .unwrap()
            .consume_step()
        {
            self.schedule.run_fixed_update(&mut self.world);
        }
        self.schedule.run_parallel(&mut self.world);
        self.schedule.run_on_event(&mut self.world);
        self.world.update_messages();
        self.world.clear_resource_changes();
        self.world.clear_component_changes();
    }

    /// Runs application ticks until an [`AppExit`] request is received.
    pub fn run(&mut self) {
        while !self
            .world
            .resource::<AppExit>()
            .is_some_and(AppExit::is_requested)
        {
            self.update();
        }
    }
}

/// Convenience constructor for registering a typed state resource.
pub fn init_state<S>(app: &mut App, state: S) -> &mut App
where
    S: PartialEq + Send + Sync + 'static,
{
    app.insert_resource(State::new(state))
}
