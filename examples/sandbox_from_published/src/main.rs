use clione::prelude::*;

struct Hp(u32);
struct Player;
struct Enemy;

struct DieadPlayer;
struct KilledEnemy(Entity);

fn main() {
    println!("Hello, clione!");
    let mut app = App::new();
    app.add_plugins(InputPlugin).add_plugins(GamePlugin).run();
}

struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.world_mut().add_messages::<KilledEnemy>();
        app.world_mut().add_messages::<DieadPlayer>();
        app.add_systems(ScheduleStage::Startup, setup_game)
            .add_systems(ScheduleStage::Update, update)
            .add_systems(ScheduleStage::Update, check_player_death)
            .add_systems(ScheduleStage::Update, check_enemy_death)
            .add_systems(ScheduleStage::Update, handle_player_death)
            .add_systems(ScheduleStage::Update, handle_enemy_death);
        let check_player_death_system_id = app.system_id(check_player_death).expect("not found");
        let handle_player_death_system_id = app.system_id(handle_player_death).expect("not found");
        app.add_dependency(check_player_death_system_id, handle_player_death_system_id)
            .expect("failed to add dependency");
        let check_enemy_death_system_id = app.system_id(check_enemy_death).expect("not found");
        let handle_enemy_death_system_id = app.system_id(handle_enemy_death).expect("not found");
        app.add_dependency(check_enemy_death_system_id, handle_enemy_death_system_id)
            .expect("failed to add dependency");
    }
}

fn setup_game(ctx: &mut SystemContext) {
    let commands = ctx.commands();

    let player = ctx.spawn();

    commands.insert(player, Player);
    commands.insert(player, Hp(100));

    let enemy = ctx.spawn();
    commands.insert(enemy, Enemy);
    commands.insert(enemy, Hp(50));
}

fn update(ctx: &mut SystemContext) {
    let mut hp_query = ctx.query_mut::<&mut Hp>();
    let player_query = ctx.query::<With<Player>>();
    let enemy_query = ctx.query::<With<Enemy>>();
    for (e, _) in player_query.iter() {
        let Some(hp) = hp_query.get_mut(e) else {
            continue;
        };
        hp.0 -= 1;
        println!("Player HP: {}", hp.0);
    }
    for (e, _) in enemy_query.iter() {
        let Some(hp) = hp_query.get_mut(e) else {
            continue;
        };
        hp.0 -= 2;
        println!("Enemy HP: {}", hp.0);
    }
}

fn check_player_death(ctx: &mut SystemContext) {
    let mut player_query = ctx.query_mut::<(&Hp, &Player)>();
    let mut message_writer = ctx.message_writer::<DieadPlayer>();
    for (entity, (hp, _)) in player_query.iter_mut() {
        if hp.0 <= 0 {
            println!("Player {:?} is dead!", entity);
            message_writer.write(DieadPlayer);
        }
    }
}

fn check_enemy_death(ctx: &mut SystemContext) {
    let mut enemy_query = ctx.query_mut::<(&Hp, &Enemy)>();
    let mut message_writer = ctx.message_writer::<KilledEnemy>();
    for (entity, (hp, _)) in enemy_query.iter_mut() {
        if hp.0 <= 0 {
            println!("Enemy {:?} is dead!", entity);
            message_writer.write(KilledEnemy(entity));
        }
    }
}

fn handle_player_death(ctx: &mut SystemContext) {
    let mut message_reader = ctx.message_reader::<DieadPlayer>().unwrap();
    let app_exit = ctx.resource_mut::<AppExit>().unwrap();
    for _ in message_reader.read() {
        println!("Handling player death...");
        app_exit.request();
    }
}

fn handle_enemy_death(ctx: &mut SystemContext) {
    let mut message_reader = ctx.message_reader::<KilledEnemy>().unwrap();
    let commands = ctx.commands();
    for message in message_reader.read() {
        println!("Handling enemy death...");
        commands.despawn(message.0);
        let new_enemy = ctx.spawn();
        commands.insert(new_enemy, Enemy);
        commands.insert(new_enemy, Hp(50));
    }
}
