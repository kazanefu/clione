use std::fmt;

use crate::World;

pub trait System {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScheduleStage {
    PreInput,
    Input,
    Update,
    PostUpdate,
    RenderPrepare,
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

#[derive(Default)]
pub struct Schedule {
    systems: Vec<(ScheduleStage, Box<dyn System>)>,
}

impl Schedule {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_system<S>(&mut self, stage: ScheduleStage, system: S)
    where
        S: System + 'static,
    {
        self.systems.push((stage, Box::new(system)));
    }

    pub fn run_stage(&mut self, stage: ScheduleStage, world: &mut World) {
        for (system_stage, system) in self.systems.iter_mut() {
            if *system_stage == stage {
                system.run(world);
            }
        }
    }

    pub fn run(&mut self, world: &mut World) {
        for stage in [
            ScheduleStage::PreInput,
            ScheduleStage::Input,
            ScheduleStage::Update,
            ScheduleStage::PostUpdate,
            ScheduleStage::RenderPrepare,
            ScheduleStage::Render,
        ] {
            self.run_stage(stage, world);
        }
    }
}
