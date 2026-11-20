use clione::prelude::*;

struct Hp(u32);
struct Player;
struct Enemy;

fn main() {
    println!("Hello, clione!");
    let mut app = App::new();
    app.add_plugins(InputPlugin);
}

struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(ScheduleStage::Startup, setup_game);
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
