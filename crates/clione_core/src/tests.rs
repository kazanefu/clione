use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Debug, PartialEq)]
struct Position(i32);

#[derive(Debug, PartialEq)]
struct Name(&'static str);

struct Marker;

#[derive(Clone, PartialEq)]
enum Screen {
    Menu,
    Game,
}

#[derive(Clone, PartialEq)]
enum Overlay {
    Hidden,
}

#[test]
fn stores_and_queries_components() {
    let mut world = World::new();
    let entity = world.spawn();

    assert!(world.insert(entity, Position(3)));
    assert_eq!(world.get::<Position>(entity), Some(&Position(3)));
    assert_eq!(world.entities_with::<Position>(), vec![entity]);
    assert_eq!(world.remove::<Position>(entity), Some(Position(3)));
    assert!(world.get::<Position>(entity).is_none());
}

#[test]
fn despawn_invalidates_old_entity_handles() {
    let mut world = World::new();
    let old_entity = world.spawn();
    world.insert(old_entity, Name("old"));

    assert!(world.despawn(old_entity));
    assert!(!world.is_alive(old_entity));
    assert!(!world.insert(old_entity, Name("invalid")));

    let new_entity = world.spawn();
    assert_ne!(old_entity, new_entity);
    assert!(world.is_alive(new_entity));
}

#[test]
fn commands_apply_structural_changes_after_systems() {
    let mut world = World::new();
    let entity = world.spawn();
    let mut commands = Commands::new();
    commands.insert(entity, Position(8));

    assert!(world.get::<Position>(entity).is_none());
    commands.apply(&mut world);
    assert_eq!(world.get::<Position>(entity), Some(&Position(8)));
}

#[test]
fn schedule_runs_systems_in_registration_order_per_stage() {
    let mut world = World::new();
    world.insert_resource(Vec::<u8>::new());
    let mut schedule = Schedule::new();
    schedule.add_system(ScheduleStage::Update, |world: &mut World| {
        world.resource_mut::<Vec<u8>>().unwrap().push(1);
    });
    schedule.add_system(ScheduleStage::Update, |world: &mut World| {
        world.resource_mut::<Vec<u8>>().unwrap().push(2);
    });

    schedule.run(&mut world);

    assert_eq!(world.resource::<Vec<u8>>().unwrap(), &vec![1, 2]);
}

#[test]
fn events_are_consumable() {
    let mut world = World::new();
    world.add_events::<u8>();
    world.add_event(1u8);
    world.add_event(2u8);

    assert_eq!(
        world
            .events_mut::<u8>()
            .unwrap()
            .drain()
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert!(world.events::<u8>().unwrap().iter().next().is_none());
}

#[test]
fn query2_matches_the_archetype_intersection() {
    let mut world = World::new();
    let matching = world.spawn();
    let only_position = world.spawn();
    world.insert(matching, Position(1));
    world.insert(matching, Name("matching"));
    world.insert(only_position, Position(2));

    let query = world.query2::<Position, Name>();
    let result: Vec<_> = query
        .iter()
        .map(|(entity, (position, name))| (entity, position.0, name.0))
        .collect();

    assert_eq!(result, vec![(matching, 1, "matching")]);
}

#[test]
fn queries_support_three_components_and_mutable_access() {
    let mut world = World::new();
    let entity = world.spawn();
    world.insert(entity, Position(1));
    world.insert(entity, Name("player"));
    world.insert(entity, Marker);

    let query = world.query::<(&Position, &Name, &Marker)>();
    assert_eq!(
        query
            .iter()
            .map(|(_, (position, name, _))| (position.0, name.0))
            .collect::<Vec<_>>(),
        vec![(1, "player")]
    );
    drop(query);

    let mut query = world.query_mut::<(&mut Position, &Name)>();
    for (_, (position, name)) in query.iter_mut() {
        position.0 += 2;
        assert_eq!(name.0, "player");
    }
    assert_eq!(world.get::<Position>(entity), Some(&Position(3)));
}

#[test]
fn dependencies_override_registration_order() {
    let mut world = World::new();
    world.insert_resource(Vec::<u8>::new());
    let mut schedule = Schedule::new();
    let second = schedule.add_system(ScheduleStage::Update, |world: &mut World| {
        world.resource_mut::<Vec<u8>>().unwrap().push(2);
    });
    let first = schedule.add_system(ScheduleStage::Update, |world: &mut World| {
        world.resource_mut::<Vec<u8>>().unwrap().push(1);
    });
    schedule.add_dependency(first, second).unwrap();

    schedule.run(&mut world);

    assert_eq!(world.resource::<Vec<u8>>().unwrap(), &vec![1, 2]);
}

#[test]
fn read_only_systems_can_run_in_parallel() {
    let mut world = World::new();
    let count = Arc::new(AtomicUsize::new(0));
    let mut schedule = Schedule::new();
    for _ in 0..2 {
        let count = Arc::clone(&count);
        schedule.add_read_only_system(ScheduleStage::Update, move |_world: &World| {
            count.fetch_add(1, Ordering::SeqCst);
        });
    }

    schedule.run_parallel(&mut world);

    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[test]
fn startup_runs_once_and_on_enter_runs_on_state_changes() {
    let mut world = World::new();
    world.insert_resource(Vec::<&'static str>::new());
    world.insert_resource(State::new(Screen::Menu));
    world.insert_resource(State::new(Overlay::Hidden));
    world.insert_resource(State::new(Overlay::Hidden));
    let mut schedule = Schedule::new();
    schedule.add_system(ScheduleStage::Startup, |world: &mut World| {
        world
            .resource_mut::<Vec<&'static str>>()
            .unwrap()
            .push("startup");
    });
    schedule.add_on_enter(Screen::Menu, |world: &mut World| {
        world
            .resource_mut::<Vec<&'static str>>()
            .unwrap()
            .push("menu");
    });
    schedule.add_on_enter(Screen::Game, |world: &mut World| {
        world
            .resource_mut::<Vec<&'static str>>()
            .unwrap()
            .push("game");
    });
    schedule.add_on_enter(Overlay::Hidden, |world: &mut World| {
        world
            .resource_mut::<Vec<&'static str>>()
            .unwrap()
            .push("overlay");
    });
    schedule.add_on_exit(Screen::Menu, |world: &mut World| {
        world
            .resource_mut::<Vec<&'static str>>()
            .unwrap()
            .push("menu-exit");
    });

    schedule.run_startup(&mut world);
    schedule.run_startup(&mut world);
    schedule.run_on_enter(&mut world);
    schedule.run_on_enter(&mut world);
    world
        .resource_mut::<State<Screen>>()
        .unwrap()
        .set(Screen::Game);
    schedule.run_on_enter(&mut world);

    assert_eq!(
        world.resource::<Vec<&'static str>>().unwrap(),
        &vec!["startup", "menu", "overlay", "menu-exit", "game"]
    );
}

#[test]
fn app_exposes_system_ids_for_dependencies() {
    let mut app = App::new();
    app.insert_resource(Vec::<&'static str>::new());
    let first = app.add_system(ScheduleStage::Update, |world: &mut World| {
        world
            .resource_mut::<Vec<&'static str>>()
            .unwrap()
            .push("first");
    });
    let second = app.add_system(ScheduleStage::Update, |world: &mut World| {
        world
            .resource_mut::<Vec<&'static str>>()
            .unwrap()
            .push("second");
    });
    app.add_dependency(first, second).unwrap().update();

    assert_eq!(
        app.world().resource::<Vec<&'static str>>().unwrap(),
        &vec!["first", "second"]
    );
}

struct CounterPlugin;

impl Plugin for CounterPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(0_u32)
            .add_systems(ScheduleStage::Update, |world: &mut World| {
                *world.resource_mut::<u32>().unwrap() += 1;
            });
    }
}

#[test]
fn app_builds_plugins() {
    let mut app = App::new();
    app.add_plugins(CounterPlugin).update();

    assert_eq!(app.world().resource::<u32>(), Some(&1));
}
