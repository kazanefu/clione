use std::{thread, time::Duration};

use clione::{Schedule, ScheduleStage, State, World};

#[derive(Debug)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Debug)]
struct Velocity {
    x: i32,
    y: i32,
}

#[derive(Debug)]
struct Player;

#[derive(Clone, PartialEq)]
enum GameState {
    Playing,
}

fn main() {
    let mut world = World::new();
    world.insert_resource(State::new(GameState::Playing));
    let player = world.spawn();
    world.insert(player, Player);
    world.insert(player, Position { x: 0, y: 0 });
    world.insert(player, Velocity { x: 1, y: 0 });

    let mut schedule = Schedule::new();
    schedule.add_system(ScheduleStage::Update, |world: &mut World| {
        let mut query = world.query_mut::<(&mut Position, &Velocity)>();
        for (_, (position, velocity)) in query.iter_mut() {
            position.x += velocity.x;
            position.y += velocity.y;
        }
    });
    schedule.add_system(ScheduleStage::Startup, |_world: &mut World| {
        println!("startup: creating sandbox world");
    });
    schedule.add_on_enter(GameState::Playing, |_world: &mut World| {
        println!("on_enter: playing");
    });
    schedule.add_read_only_system(ScheduleStage::RenderPrepare, |world: &World| {
        let query = world.query::<(&Position, &Player)>();
        for (_, (position, _)) in query.iter() {
            println!("player: ({}, {})", position.x, position.y);
        }
    });

    schedule.run_startup(&mut world);

    println!("sandbox: running 10 fixed-timestep ECS ticks");
    for _ in 0..10 {
        schedule.run_on_enter::<GameState>(&mut world);
        schedule.run_parallel(&mut world);
        thread::sleep(Duration::from_millis(100));
    }
}
