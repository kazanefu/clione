use super::*;

#[derive(Debug, PartialEq)]
struct Position(i32);

#[derive(Debug, PartialEq)]
struct Name(&'static str);

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
