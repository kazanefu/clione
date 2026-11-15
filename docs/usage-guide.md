# Clione 利用ガイド

Clione は、端末上で動作するゲームを ECS で構築するための Rust ライブラリです。
ゲームの状態は `World` の Entity、Component、Resource、Message として管理し、
`App` が Schedule を毎 tick 実行します。

## 依存関係と Prelude

ゲーム側では facade crate を依存関係に追加し、通常は次の import を使用します。

```rust
use clione::prelude::*;
```

低レベルの ECS だけを使う場合は `clione_core` を直接利用できます。

## App とゲームループ

```rust
use clione::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_systems(ScheduleStage::Startup, |world: &mut World| {
        world.insert_resource(0_u32);
    });
    app.add_systems(ScheduleStage::Update, |world: &mut World| {
        *world.resource_mut::<u32>().unwrap() += 1;
    });

    // テストや外部ループでは1 tickだけ実行する。
    app.update_with_delta(std::time::Duration::from_millis(16));
    // 実際のゲームでは終了要求まで app.run() を実行する。
}
```

`update()` は実時間の差分を使い、`update_with_delta()` は決定的なテストに使います。
`App::run()` はStartupを一度実行した後、通常のSchedule、FixedUpdate、イベント処理を
繰り返します。`AppExit`を送るとループを終了できます。

## Plugin

機能単位の初期化は `Plugin` にまとめます。

```rust
struct ScorePlugin;

impl Plugin for ScorePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(0_u32);
        app.add_systems(ScheduleStage::Update, |world: &mut World| {
            *world.resource_mut::<u32>().unwrap() += 1;
        });
    }
}
```

## Entity と Component

Component は `Send + Sync + 'static` の値として Entity に追加します。

```rust
#[derive(Debug, PartialEq)]
struct Position { x: i32, y: i32 }

let entity = app.world_mut().spawn();
app.world_mut().insert(entity, Position { x: 1, y: 2 });
assert_eq!(app.world().get::<Position>(entity).unwrap().x, 1);
```

`despawn`、`remove::<T>`、`is_alive` も `World` から利用できます。削除された Entity
の番号が再利用されても世代番号が変わるため、古い Entity は有効になりません。

## Query

読み取り Query と可変 Query を使って Component を走査します。

```rust
app.add_param_system(ScheduleStage::Update, |context: &mut SystemContext| {
    let mut positions = context.query_mut::<&mut Position>();
    for (_entity, position) in positions.iter_mut() {
        position.x += 1;
    }
});
```

複数 Component は tuple で指定できます。

```rust
let mut query = context.query_mut::<(&mut Position, &Velocity)>();
for (_entity, (position, velocity)) in query.iter_mut() {
    position.x += velocity.x;
}
```

同一システムで複数の Query や Resource を取得する場合は `SystemContext` を使います。
同じ Component / Resource に対する読み書き競合は、パラメータ取得時に検出されます。

### Added / Changed フィルタ

`Added<T>` は現在の変更世代で Component が追加された Entity、
`Changed<T>` は Component が追加または変更された Entity を選択します。

```rust
app.add_param_system(ScheduleStage::Update, |context: &mut SystemContext| {
    let query = context.query::<(&Position, Added<Position>)>();
    for (entity, (position, _)) in query.iter() {
        println!("{entity:?}: {} {}", position.x, position.y);
    }
});
```

Component の変更世代は Entity と Component 型ごとに管理され、tick の終了時に
変更フラグがリセットされます。`World::component_change_generation::<T>(entity)`
で現在の世代番号も確認できます。

`With<T>` と `Without<T>` はComponentの存在だけを条件にするフィルタです。

```rust
let query = context.query::<(&Position, With<Player>)>();
let query = context.query::<(&Position, Without<Hidden>)>();
```

## Resource

Resource は World に型ごとに一つだけ存在する共有データです。

```rust
app.insert_resource(Time::default());
app.add_param_system(ScheduleStage::Update, |context: &mut SystemContext| {
    if let Some(time) = context.resource::<Time>() {
        let delta = time.delta();
        let _ = delta;
    }
});
```

`resource::<T>()` は読み取り、`resource_mut::<T>()` は可変アクセスです。

構造変更をエラーとして扱いたい場合は `try_insert`、`try_remove`、`try_despawn` を
使います。これらは `WorldError::InvalidEntity` と
`WorldError::MissingComponent` を返します。既存のbool / Option APIも互換性のため
利用できます。

## Commands と遅延変更

System 実行中の構造変更は `Commands` に積み、System 終了後に適用します。

```rust
app.add_param_system(ScheduleStage::Update, |context: &mut SystemContext| {
    let entity = context.spawn();
    context.commands().insert(entity, Position { x: 0, y: 0 });
});
```

`insert`、`remove`、`despawn`、予約 Entity の生成が利用できます。Commands の変更は
後続 System から見えるよう、各 System の後に flush されます。

## Messages

`Messages<T>` は tick 単位で配送するデータ付きメッセージです。`Events<T>` は互換用の
旧名称です。

```rust
#[derive(Debug)]
struct Damage(u32);

app.add_param_system(ScheduleStage::Update, |context: &mut SystemContext| {
    let mut writer = context.message_writer::<Damage>();
    writer.write(Damage(1));
});

app.add_param_system(ScheduleStage::Update, |context: &mut SystemContext| {
    if let Some(mut reader) = context.message_reader::<Damage>() {
        for damage in reader.read() {
            let _amount = damage.0;
        }
    }
});
```

Reader の既読位置は System ID とメッセージ型ごとに独立して保持されます。
`read()` は未読メッセージを返す iterator です。通常のメッセージは tick の最後に
クリアされます。`OnMessage::<T>::new()`（または `OnEvent::<T>::new()`）を登録すると、
メッセージがある tick だけ System を実行できます。

## State とライフサイクル

```rust
#[derive(Clone, PartialEq)]
enum GameState { Menu, Playing }

let mut app = App::new();
init_state(&mut app, GameState::Menu);
app.add_systems(OnEnter(GameState::Playing), |world: &mut World| {
    let _ = world.spawn();
});
app.add_systems(OnExit(GameState::Playing), |_world: &mut World| {});
```

State の変更は遷移として記録され、対応する `OnExit`、`OnEnter` が通常の tick 内で
実行されます。

## 実行順序と FixedUpdate

`ScheduleStage` には Startup、PreInput、Input、Update、FixedUpdate、PostUpdate、
RenderPrepare、Render があります。`add_system` / `add_param_system` は `SystemId` を
返し、依存関係を明示できます。

```rust
let input = app.add_system(ScheduleStage::Input, input_system);
let update = app.add_system(ScheduleStage::Update, update_system);
app.add_dependency(input, update).unwrap();
```

`FixedTime` の蓄積時間が step を超えた回数だけ FixedUpdate が実行されます。
`Time`、`VirtualTime`、`FixedTime` はゲーム速度、一時停止、固定刻みの制御に使います。

## 直接 World / Schedule を使う

App が不要なライブラリやテストでは、World と Schedule を直接組み合わせられます。

```rust
let mut world = World::new();
let mut schedule = Schedule::new();
schedule.add_system(ScheduleStage::Update, |_world: &mut World| {});
schedule.run(&mut world);
```

## テストと注意点

ゲームロジックは端末 I/O から分離し、`World` と `update_with_delta()` を使うと
決定的にテストできます。端末入力、描画、raw mode、alternate screen は `clione_input`、
`clione_render`、`clione_terminal` の責務であり、core は端末へ直接アクセスしません。

`SystemContext` 内部では複数の参照を安全に共存させるためアクセスを記録しています。
同一型への可変アクセスを複数取得する設計は避け、必要なら一つの Query にまとめてください。

## Core標準型

端末非依存の座標・描画・入力型はcoreから利用できます。

```rust
let mut frame = Frame::new(Size::new(80, 24));
frame.set(Point::new(2, 1), Cell::new('@'));
let event = InputEvent::KeyDown(Key::Char('q'));
let _ = (event, TerminalSize(Size::new(80, 24)), Layer(1));
```

`Frame::set`は範囲外なら`false`を返し、`Frame::get`は範囲外なら`None`を返します。
端末I/OやANSI変換はcoreでは行わず、terminal/renderクレートが担当します。
