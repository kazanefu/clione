use std::{thread, time::Duration};

use clione::prelude::*;

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
    Exit,
}

fn main() {
    let mut app = App::new();
    app.insert_resource(State::new(GameState::Playing));
    let player = app.world_mut().spawn();
    app.world_mut().insert(player, Player);
    app.world_mut().insert(player, Position { x: 0, y: 0 });
    app.world_mut().insert(player, Velocity { x: 1, y: 2 });

    app.add_systems(ScheduleStage::Update, |world: &mut World| {
        let mut query = world.query_mut::<(&mut Position, &Velocity)>();
        for (_, (position, velocity)) in query.iter_mut() {
            position.x += velocity.x;
            position.y += velocity.y;
        }
    });
    app.add_systems(ScheduleStage::Startup, |_world: &mut World| {
        println!("startup: creating sandbox world");
    });
    app.add_systems(OnEnter(GameState::Playing), |_world: &mut World| {
        println!("on_enter: playing");
    });
    app.add_read_only_system(ScheduleStage::RenderPrepare, |world: &World| {
        let query = world.query::<(&Position, &Player)>();
        for (_, (position, _)) in query.iter() {
            println!("player: ({}, {})", position.x, position.y);
        }
    });

    app.add_systems(ScheduleStage::Update, |world: &mut World| {
        let state = world.resource::<State<GameState>>().unwrap();
        if state.get() == &GameState::Playing {
            let query = world.query::<(&Position, &Player)>();
            for (_, (position, _)) in query.iter() {
                if position.x > 10 {
                    println!("requesting exit");
                    let state = world.resource_mut::<State<GameState>>().unwrap();
                    state.set(GameState::Exit);
                }
            }
        }
    });

    app.add_systems(OnExit(GameState::Playing), |world: &mut World| {
        println!("on_exit: playing");
        let app_exit = world.resource_mut::<AppExit>().unwrap();
        app_exit.request();
    });

    println!("sandbox: running 30 fixed-timestep ECS ticks");
    for _ in 0..30 {
        app.update();
        thread::sleep(Duration::from_millis(100));
    }
}
