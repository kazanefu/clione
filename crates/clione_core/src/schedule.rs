use std::{fmt, sync::Arc};

use crate::World;

/// A mutable system that runs against the ECS world.
pub trait System {
    /// Executes the system against a mutable world.
    fn run(&mut self, world: &mut World);
}

impl<F> System for F
where
    F: FnMut(&mut World),
{
    fn run(&mut self, world: &mut World) {
        self(world);
    }
}

/// A system that only observes the world and can run in parallel.
pub trait ReadOnlySystem: Send + Sync {
    /// Executes the system while observing an immutable world.
    fn run(&self, world: &World);
}

impl<F> ReadOnlySystem for F
where
    F: Fn(&World) + Send + Sync,
{
    fn run(&self, world: &World) {
        self(world);
    }
}

/// Stable handle for a mutable system in a schedule.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SystemId(usize);

/// The fixed execution phases of a game tick.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScheduleStage {
    /// Runs before input translation.
    PreInput,
    /// Translates external input into game intent.
    Input,
    /// Runs simulation and game rules.
    Update,
    /// Applies cleanup and deferred structural changes.
    PostUpdate,
    /// Builds render data from the world.
    RenderPrepare,
    /// Emits render data to the terminal.
    Render,
}

impl fmt::Display for ScheduleStage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::PreInput => "PreInput",
            Self::Input => "Input",
            Self::Update => "Update",
            Self::PostUpdate => "PostUpdate",
            Self::RenderPrepare => "RenderPrepare",
            Self::Render => "Render",
        };
        formatter.write_str(name)
    }
}

/// Errors returned while building a schedule dependency graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScheduleError {
    /// A dependency refers to a system that is not in this schedule.
    UnknownSystem,
    /// A dependency would create a cycle.
    Cycle,
    /// A dependency crosses fixed execution stages in the wrong direction.
    InvalidStageOrder,
}

struct SystemNode {
    stage: ScheduleStage,
    system: Box<dyn System>,
    dependencies: Vec<SystemId>,
}

struct ReadOnlyNode {
    stage: ScheduleStage,
    system: Arc<dyn ReadOnlySystem>,
}

/// An ordered collection of ECS systems.
#[derive(Default)]
pub struct Schedule {
    systems: Vec<SystemNode>,
    read_only_systems: Vec<ReadOnlyNode>,
}

impl Schedule {
    /// Creates an empty schedule.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a mutable system and returns its handle.
    pub fn add_system<S>(&mut self, stage: ScheduleStage, system: S) -> SystemId
    where
        S: System + 'static,
    {
        let id = SystemId(self.systems.len());
        self.systems.push(SystemNode {
            stage,
            system: Box::new(system),
            dependencies: Vec::new(),
        });
        id
    }

    /// Adds a read-only system for execution by `run_parallel`.
    pub fn add_read_only_system<S>(&mut self, stage: ScheduleStage, system: S)
    where
        S: ReadOnlySystem + 'static,
    {
        self.read_only_systems.push(ReadOnlyNode {
            stage,
            system: Arc::new(system),
        });
    }

    /// Makes `before` run before `after`.
    pub fn add_dependency(
        &mut self,
        before: SystemId,
        after: SystemId,
    ) -> Result<(), ScheduleError> {
        let Some(before_stage) = self.systems.get(before.0).map(|node| node.stage) else {
            return Err(ScheduleError::UnknownSystem);
        };
        let Some(after_stage) = self.systems.get(after.0).map(|node| node.stage) else {
            return Err(ScheduleError::UnknownSystem);
        };
        if before_stage as u8 > after_stage as u8 {
            return Err(ScheduleError::InvalidStageOrder);
        }
        if before == after || self.systems[after.0].dependencies.contains(&before) {
            return Err(ScheduleError::Cycle);
        }
        self.systems[after.0].dependencies.push(before);
        if self.execution_order(after_stage).is_err() {
            self.systems[after.0].dependencies.pop();
            return Err(ScheduleError::Cycle);
        }
        Ok(())
    }

    /// Runs one stage in dependency order.
    pub fn run_stage(&mut self, stage: ScheduleStage, world: &mut World) {
        let order = self
            .execution_order(stage)
            .expect("schedule dependencies must be valid");
        for id in order {
            self.systems[id.0].system.run(world);
        }
    }

    /// Runs all mutable systems serially.
    pub fn run(&mut self, world: &mut World) {
        for stage in ScheduleStage::ALL {
            self.run_stage(stage, world);
        }
    }

    /// Runs mutable systems serially and read-only systems concurrently per stage.
    pub fn run_parallel(&mut self, world: &mut World) {
        for stage in ScheduleStage::ALL {
            self.run_stage(stage, world);
            let systems: Vec<_> = self
                .read_only_systems
                .iter()
                .filter(|node| node.stage == stage)
                .map(|node| Arc::clone(&node.system))
                .collect();
            let shared_world: &World = &*world;
            std::thread::scope(|scope| {
                for system in systems {
                    scope.spawn(move || system.run(shared_world));
                }
            });
        }
    }

    fn execution_order(&self, stage: ScheduleStage) -> Result<Vec<SystemId>, ScheduleError> {
        let ids: Vec<_> = self
            .systems
            .iter()
            .enumerate()
            .filter_map(|(index, node)| (node.stage == stage).then_some(SystemId(index)))
            .collect();
        let mut order = Vec::with_capacity(ids.len());
        let mut remaining = ids;
        while !remaining.is_empty() {
            let next = remaining.iter().position(|id| {
                self.systems[id.0]
                    .dependencies
                    .iter()
                    .all(|dependency| !remaining.contains(dependency))
            });
            let Some(index) = next else {
                return Err(ScheduleError::Cycle);
            };
            order.push(remaining.remove(index));
        }
        Ok(order)
    }
}

impl ScheduleStage {
    const ALL: [Self; 6] = [
        Self::PreInput,
        Self::Input,
        Self::Update,
        Self::PostUpdate,
        Self::RenderPrepare,
        Self::Render,
    ];
}
