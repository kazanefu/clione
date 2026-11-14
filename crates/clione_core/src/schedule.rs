use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt,
    sync::Arc,
};

use crate::{State, World};

type Transition = Option<Box<dyn Any + Send + Sync>>;
type TransitionConsumer = Box<dyn Fn(&mut World) -> Option<Transition> + Send + Sync>;
type PreviousStateMatcher = Box<dyn Fn(&dyn Any) -> bool + Send + Sync>;

/// Registers systems that run when a state is entered.
pub struct OnEnter<S>(pub S);

/// Registers systems that run when a state is exited.
pub struct OnExit<S>(pub S);

/// Type-erased target used by [`Schedule::add_systems`].
pub enum ScheduleTarget {
    /// A regular execution stage.
    Stage(ScheduleStage),
    /// A state-enter lifecycle target.
    OnEnter {
        /// Type identifier of the state resource.
        state_type: TypeId,
        /// Consumes the pending state transition.
        consume_transition: TransitionConsumer,
        /// Checks whether the current state matches.
        matches: Box<dyn Fn(&World) -> bool + Send + Sync>,
    },
    /// A state-exit lifecycle target.
    OnExit {
        /// Type identifier of the state resource.
        state_type: TypeId,
        /// Checks whether the previous state matches.
        matches: PreviousStateMatcher,
    },
}

/// A target accepted by [`Schedule::add_systems`].
pub trait IntoScheduleTarget {
    /// Converts the public target into an internal schedule target.
    fn into_schedule_target(self) -> ScheduleTarget;
}

impl IntoScheduleTarget for ScheduleStage {
    fn into_schedule_target(self) -> ScheduleTarget {
        ScheduleTarget::Stage(self)
    }
}

impl<S> IntoScheduleTarget for OnEnter<S>
where
    S: PartialEq + Clone + Send + Sync + 'static,
{
    fn into_schedule_target(self) -> ScheduleTarget {
        let state = self.0;
        ScheduleTarget::OnEnter {
            state_type: TypeId::of::<S>(),
            consume_transition: Box::new(|world| {
                world
                    .resource_mut::<State<S>>()
                    .and_then(|state| state.take_transition())
                    .map(|previous| {
                        previous.map(|previous| Box::new(previous) as Box<dyn Any + Send + Sync>)
                    })
            }),
            matches: Box::new(move |world| {
                world
                    .resource::<State<S>>()
                    .is_some_and(|current| current.get() == &state)
            }),
        }
    }
}

impl<S> IntoScheduleTarget for OnExit<S>
where
    S: PartialEq + Clone + Send + Sync + 'static,
{
    fn into_schedule_target(self) -> ScheduleTarget {
        let state = self.0;
        ScheduleTarget::OnExit {
            state_type: TypeId::of::<S>(),
            matches: Box::new(move |previous| {
                previous
                    .downcast_ref::<S>()
                    .is_some_and(|previous| previous == &state)
            }),
        }
    }
}

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
    /// Runs once before the first game tick.
    Startup,
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
            Self::Startup => "Startup",
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

struct OnEnterNode {
    state_type: TypeId,
    consume_transition: TransitionConsumer,
    matches: Box<dyn Fn(&World) -> bool + Send + Sync>,
    system: Box<dyn System>,
}

struct OnExitNode {
    state_type: TypeId,
    matches: PreviousStateMatcher,
    system: Box<dyn System>,
}

/// An ordered collection of ECS systems.
#[derive(Default)]
pub struct Schedule {
    systems: Vec<SystemNode>,
    read_only_systems: Vec<ReadOnlyNode>,
    on_enter_systems: Vec<OnEnterNode>,
    on_exit_systems: Vec<OnExitNode>,
    startup_ran: bool,
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

    /// Adds a system to a stage or state lifecycle target.
    pub fn add_systems<T, F>(&mut self, target: T, system: F)
    where
        T: IntoScheduleTarget,
        F: System + 'static,
    {
        match target.into_schedule_target() {
            ScheduleTarget::Stage(stage) => {
                self.systems.push(SystemNode {
                    stage,
                    system: Box::new(system),
                    dependencies: Vec::new(),
                });
            }
            ScheduleTarget::OnEnter {
                state_type,
                consume_transition,
                matches,
            } => self.on_enter_systems.push(OnEnterNode {
                state_type,
                consume_transition,
                matches,
                system: Box::new(system),
            }),
            ScheduleTarget::OnExit {
                state_type,
                matches,
            } => self.on_exit_systems.push(OnExitNode {
                state_type,
                matches,
                system: Box::new(system),
            }),
        }
    }

    /// Adds a system that runs when the typed state enters `state`.
    pub fn add_on_enter<S, F>(&mut self, state: S, system: F)
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        F: System + 'static,
    {
        self.add_systems(OnEnter(state), system);
    }

    /// Adds a system that runs when the typed state exits `state`.
    pub fn add_on_exit<S, F>(&mut self, state: S, system: F)
    where
        S: PartialEq + Clone + Send + Sync + 'static,
        F: System + 'static,
    {
        self.add_systems(OnExit(state), system);
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

    /// Runs startup systems once. Repeated calls are no-ops.
    pub fn run_startup(&mut self, world: &mut World) {
        if self.startup_ran {
            return;
        }
        self.run_stage(ScheduleStage::Startup, world);
        self.startup_ran = true;
    }

    /// Runs all systems registered for state enter and exit transitions.
    ///
    /// Each state type is checked once, so callers do not need to name every
    /// registered `State<S>` in the game loop.
    pub fn run_on_enter(&mut self, world: &mut World) {
        let systems = &self.on_enter_systems;
        let mut transitions = HashMap::new();
        for node in systems {
            if transitions.contains_key(&node.state_type) {
                continue;
            }
            if let Some(previous) = (node.consume_transition)(world) {
                transitions.insert(node.state_type, previous);
            }
        }

        let mut systems = std::mem::take(&mut self.on_exit_systems);
        let matching: Vec<_> = (0..systems.len())
            .filter(|&index| {
                transitions
                    .get(&systems[index].state_type)
                    .and_then(|previous| previous.as_ref())
                    .is_some_and(|previous| (systems[index].matches)(previous.as_ref()))
            })
            .collect();
        for index in matching {
            systems[index].system.run(world);
        }
        self.on_exit_systems = systems;

        let mut systems = std::mem::take(&mut self.on_enter_systems);
        let matching: Vec<_> = (0..systems.len())
            .filter(|&index| {
                transitions.contains_key(&systems[index].state_type)
                    && (systems[index].matches)(world)
            })
            .collect();
        for index in matching {
            systems[index].system.run(world);
        }
        self.on_enter_systems = systems;
    }

    /// Runs all mutable systems serially.
    pub fn run(&mut self, world: &mut World) {
        for stage in ScheduleStage::TICK {
            self.run_stage(stage, world);
        }
    }

    /// Runs mutable systems serially and read-only systems concurrently per stage.
    pub fn run_parallel(&mut self, world: &mut World) {
        for stage in ScheduleStage::TICK {
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
    const TICK: [Self; 6] = [
        Self::PreInput,
        Self::Input,
        Self::Update,
        Self::PostUpdate,
        Self::RenderPrepare,
        Self::Render,
    ];
}
