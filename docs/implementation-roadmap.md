# CLIゲームエンジン実装方針

## 目的

`clione` は、端末上に ASCII アートを描画するリアルタイム CLI ゲーム向けの Rust ライブラリとする。ゲーム固有のロジックは利用者が実装し、エンジンは次の共通処理を提供する。

- 固定または可変のゲームループ
- キーボード入力の取得とゲームイベントへの変換
- 端末サイズ・カーソル・描画モードの管理
- 画面全体または差分の ASCII/ANSI 描画
- ECS のワールド、コンポーネント、リソース、システムによるゲーム状態管理
- テスト可能な時間・入力・描画の抽象化

当面は現在のクレート構成と依存関係を維持する。外部クレートの追加や `Cargo.toml` の依存関係変更は行わず、必要な OS/端末依存処理は各クレートの境界内に閉じ込める。

## クレートの責務と依存関係

```text
clione_core
├── clione_input
├── clione_terminal
└── clione_render ── clione_terminal

clione (facade)
├── clione_core
├── clione_input
├── clione_render
└── clione_terminal
```

## ECS 方針

ECS は外部クレートに依存せず、まず `clione_core` に最小実装を持つ。ゲームの状態は `World` に集約し、ゲーム固有の状態を大きな `GameState` 構造体へまとめない。`clione` は ECS のスケジュールを実行し、各システムが入力・更新・描画データを扱う。

```text
World
├── Entities
├── Component storages
├── Resources
└── Events

Schedule
├── Input systems
├── Simulation systems
├── Cleanup systems
└── Render systems
```

### ECS の最小モデル

- `Entity`: 世代番号を含む軽量なハンドル。削除済み ID の再利用で古い参照が有効にならないようにする
- `Component`: `Send`/`Sync` を前提にせず、まずは `'static` な型を登録できる型消去ストレージとして設計する
- `World`: エンティティの生成/削除、コンポーネントの挿入/取得/除去、リソース、イベントを管理する
- `Resource`: 時刻、入力、端末サイズ、設定、乱数など、ワールド全体で一つだけ存在するデータ
- `Query`: 指定コンポーネントを持つエンティティを走査する読み取り/書き込み API
- `Commands`: システム中の entity 操作を遅延適用するバッファ。走査中の構造変更を禁止または安全に分離する
- `Event<T>`: 入力、終了要求、リサイズ、衝突などを tick 単位で配送する一方向のイベントチャネル
- `System`: `World` のデータを読み書きして処理する関数またはトレイト
- `Schedule`: システムの順序、実行段階、将来の依存関係を定義する実行器

初期版は API の安全性と決定性を優先し、archetype ごとの dense column storage を基本実装とする。将来的には archetype 遷移、query キャッシュ、borrow access 集約、並列 system 実行を段階的に最適化する。

### ECS のシステム段階

固定 timestep の 1 tick を次の段階に分ける。

1. `PreInput`: 前 tick のイベントを消費し、入力リソースを更新する
2. `Input`: 入力イベントをゲーム用コンポーネントや意図（intent）へ変換する
3. `Update`: 移動、AI、ルールなどのシミュレーションを実行する
4. `PostUpdate`: コマンド適用、寿命切れ entity の削除、イベントの整理を行う
5. `RenderPrepare`: ワールドから `Frame` を生成する
6. `Render`: `Frame` を端末へ出力する

同じ段階内の実行順は登録順で決定し、システムの暗黙の順序に依存しない。並列実行は初期版の対象外とする。

### `clione_core`

ゲーム本体からも他のクレートからも利用する、端末に依存しないデータモデルを置く。

- `Entity`、世代管理、エンティティの生成/削除
- `Component` storage、`Query`、`Res`/`ResMut` 相当の取得 API
- `World`、`Resource`、`Event`、`Commands`
- `System`、`SystemSet`、`Schedule`、実行段階
- `Time`、`InputState`、`TerminalSize` などの標準リソース
- `UpdateContext`（経過時間、tick、入力、画面サイズなど）
- `Duration`、`Tick`、`FrameRate` などの時間・更新設定
- `Point`、`Size`、`Rect`、`Color`、`Cell` などの基本型
- `Key`、`InputEvent`、`Action` など、入力とゲーム処理の共通型
- `Frame`、`Layer`、`DrawCommand` など、描画結果の端末非依存表現
- `EngineError` と各クレートが利用できるエラー境界
- 端末 I/O、時刻取得、スリープを直接行わない純粋なロジック

`core` は端末 I/O を行わず、ECS のデータ構造と実行契約だけを提供する。システム実行中の borrow conflict、存在しない component、無効な entity、二重登録などは、黙って無視せず明示的なエラーまたは型で扱う。

### `clione_input`

端末から得た入力を、ゲームが扱えるイベント列へ変換する。

- raw/cooked モードを意識した入力の開始・終了
- 文字入力、矢印キー、特殊キー、Ctrl 系入力の正規化
- `KeyDown`、`KeyUp`、`Text`、`Resize`、`Quit` のイベント化
- 読み取り待ち、タイムアウト、EOF、未知のシーケンスの扱い
- 入力を一度に取り出す `InputSource` と、テスト用のキュー実装
- 入力バッファのクリアとイベント順序の保証
- `InputEvent` を `World` のイベントリソースへ投入する input schedule 用アダプター

端末のバイト列を `core` の公開型へ変換するところまでを担当し、ゲームルールやキー割り当ては担当しない。

### `clione_terminal`

端末の状態変更と低レベル I/O を担当する。

- ANSI エスケープによるカーソル移動・表示/非表示
- alternate screen の開始・復元
- raw モードの開始・終了
- 端末サイズの取得
- stdout への書き込み、flush、I/O エラーの伝播
- `TerminalSession` による RAII 復元
- ANSI を出力せずに検証できる `TerminalWriter` 抽象とテスト用 writer

途中で panic やエラーが発生しても端末が壊れた状態で残らないよう、開始した状態を `Drop` で可能な範囲まで復元する。復元処理の失敗を隠さず、明示的な API ではエラーとして返す。

### `clione_render`

`core::Frame` を端末へ出力可能な描画コマンドへ変換する。

- 文字セル単位の画面バッファ
- ASCII アート、空白、前景/背景色、スタイルの表現
- 座標の範囲外や幅 0/高さ 0 の扱い
- レイヤー順、重なり、クリッピング
- 前フレームとの差分計算
- ANSI コマンド列へのシリアライズ
- 全画面描画と差分描画の切り替え
- 行末の余分な出力を消す処理
- `World` の描画用 component（例: `Transform`、`Sprite`、`ZIndex`）を query して `Frame` を作る render system

描画器は直接 stdout を所有せず、`clione_terminal` の writer/session に書き込む。まずは全画面描画を完成させ、差分描画は同じ `Frame` 契約の上に追加する。

### `clione`

利用者が最初に依存する facade crate とする。

- 各クレートの主要な型の再エクスポート
- `Engine` / `App` / `World` / `Schedule` の構築 API
- 入力取得、更新、描画、フレーム待機を統合したゲームループ
- 終了要求、端末サイズ変更、エラーの扱い
- 設定（更新間隔、描画方式、終了キー、初期画面）
- ゲーム固有の component、resource、system を登録する API

facade に独自の重複型を作らず、公開型は可能な限り `core` の型を再利用する。`Game` トレイトを中心にせず、`App::add_systems` と `World` を中心に組み立てる。

### `examples/sandbox`

API の実行例かつ手動確認用の最小ゲームとする。

- `Position`、`Velocity`、`Player`、`Wall`、`Renderable` などの component を定義する
- プレイヤーを矢印キーで動かす input/update system を登録する
- ASCII のプレイヤー、壁、情報 HUD を描画する
- `q` または終了イベントで正常終了する
- ウィンドウ/端末サイズ変更を処理する
- ゲームループ、入力、描画を facade API だけで組み立てる

## 実装フェーズ

### Phase 0: 契約を決める

- [ ] `World` が所有するデータと、system が所有してよい一時データを分ける
- [ ] `Entity` の世代、削除、再利用、無効ハンドルの仕様を決める
- [ ] component の登録、型 ID、storage、query の borrow 規則を決める
- [ ] resource と event の寿命（tick 単位/アプリケーション全体）を決める
- [ ] `System` の引数、失敗時の扱い、登録順と段階を決める
- [ ] `Commands` の適用時点と、システム間の構造変更の仕様を決める
- [ ] `Frame` が「画面全体」か「描画命令列」かを決める
- [ ] 座標系、端末セルの幅、画面外描画の仕様を決める
- [ ] 入力イベントの意味、順序、未知入力の扱いを決める
- [ ] エラー型と、端末復元を保証する境界を決める

### Phase 1: `clione_core` を実装する

- [ ] ダミーの `add` 関数と雛形テストを削除する
- [ ] `Entity` と世代付き entity allocator を実装する
- [ ] 型ごとの component storage と insert/get/remove API を実装する
- [ ] `World` の entity/component/resource/event API を実装する
- [ ] `Query` の immutable/mutable access と filter を実装する
- [ ] `Commands` の spawn/insert/remove/despawn と遅延適用を実装する
- [ ] `System`、system parameter、`Schedule`、段階実行を実装する
- [ ] 基本の座標・サイズ・矩形型と不変条件を実装する
- [ ] `Key`、`InputEvent`、終了/リサイズイベントを実装する
- [ ] `Time`、入力、端末サイズなどの標準 resource を実装する
- [ ] `Cell`、`Frame`、レイヤーまたは描画コマンドを実装する
- [ ] `Frame` のクリア、セル設定、矩形描画、境界チェックを実装する
- [ ] entity 世代、query、commands、system 順序のユニットテストを追加する
- [ ] `Frame` と入力イベントのユニットテストを追加する

完了条件は、端末を使わずに component の spawn、query、更新 system、render system を決定的にテストできること。

Message reader は `World` 内でシステム IDとメッセージ型ごとに読み取り位置を保持する。
そのため、同じメッセージ型を読む複数システムは互いの既読状態に影響せず、
`MessageReader::read()` は各システムの未読メッセージだけをイテレーターとして返す。

### Phase 2: `clione_terminal` を実装する

- [ ] `TerminalWriter` と `TerminalSize` の抽象を実装する
- [ ] alternate screen/raw mode/cursor の状態操作を実装する
- [ ] `TerminalSession` の開始・終了・復元を実装する
- [ ] write/flush エラーを `EngineError` 系へ変換する
- [ ] モック writer で出力順序と復元処理をテストする
- [ ] 非対応端末やサイズ取得失敗の扱いを定義する

### Phase 3: `clione_input` を実装する

- [ ] 入力ソースの抽象とテスト用のメモリ入力を実装する
- [ ] 通常キー・矢印キー・Ctrl キー・終了キーを解析する
- [ ] escape sequence のバッファリングと不完全入力を処理する
- [ ] タイムアウトと入力なしを、ゲームループを止めない形で返す
- [ ] リサイズイベントを入力イベントへ変換する
- [ ] input system 用に `InputEvent` を `World` の event/resource へ投入する
- [ ] キー割り当てを端末解析から分離し、ゲーム側 system で action/component へ変換する
- [ ] 各入力バイト列を対象にした解析テストを追加する

### Phase 4: `clione_render` を実装する

- [ ] `Frame` をセル/行へ変換する全画面レンダラーを実装する
- [ ] 文字、色、スタイル、カーソル位置の ANSI 変換を実装する
- [ ] 画面外セル、空白、行末、改行の出力仕様を固定する
- [ ] 前フレームとの差分レンダラーを実装する
- [ ] ANSI 文字列ではなく中間コマンド列を検証するテストを追加する
- [ ] 小さい固定フレームで全画面/差分出力を比較する
- [ ] `Transform`、表示文字、レイヤー順を query する render system を実装する
- [ ] entity の despawn/非表示 component を描画対象から除外する

### Phase 5: `clione` のゲームループを実装する

- [ ] builder で `World`、system、schedule、端末・入力・renderer・設定を注入できるようにする
- [ ] fixed timestep を基本とし、経過時間を `update` へ渡す
- [ ] 1 tick 内の input/update/post-update/render の順序を固定する
- [ ] 各段階で `Commands` を適用し、次段階から変更を参照できるようにする
- [ ] 描画に時間がかかった場合の遅延・tick 蓄積を制御する
- [ ] 終了要求、エラー、panic 後の端末復元を扱う
- [ ] テスト用の clock/input/writer でループを決定的に検証する
- [ ] system panic/error 時に端末復元と world の後処理が行われる境界を検証する

### Phase 6: sandbox と品質整備

- [ ] sandbox を実際に操作できる最小ゲームへ置き換える
- [ ] 起動、操作、終了、リサイズを手動確認する
- [ ] 公開 API に rustdoc と使用例を追加する
- [ ] `cargo fmt --all`、`cargo clippy --workspace --all-targets`、`cargo test --workspace` を通す
- [ ] Windows、Unix 系端末で端末復元とキー入力の差異を確認する
- [ ] 互換性を壊す API 変更を避け、0.1 系では仕様を小さく保つ

## ゲームループの基準設計

基本の 1 tick は次の順序とする。

1. 入力を非ブロッキングで収集し、`World` の event/resource へ投入する
2. `PreInput`/`Input` system を実行する
3. `Update` system を経過時間付きで実行する
4. `PostUpdate` system と `Commands` を適用する
5. `RenderPrepare` system が query 結果から次の `Frame` を作る
6. `Render` system が端末へ描画し、flush する
7. tick イベントを整理し、次の tick まで待機する

ゲームロジックは端末 I/O を直接呼ばず、`World` の component/resource/event を system parameter として受け取る。clock、input source、writer は差し替え可能にし、リアルタイム実行と決定的なテストを同じ schedule で扱える。

## 非機能要件と注意点

- ANSI エスケープをゲームロジックへ漏らさない
- 端末の終了処理を `Drop` 任せにせず、明示的な `shutdown` も提供する
- UTF-8 文字を扱う場合の表示幅は ASCII と分けて仕様化する。初期版は ASCII を主対象とする
- 画面サイズ変更時は panic せず、`Resize` と新しい `TerminalSize` をゲームへ通知する
- 入力や描画のエラーを空の成功として扱わない
- system の実行順と component/resource の競合を明示し、暗黙のグローバル状態を作らない
- query の走査中に world の構造を直接変更せず、`Commands` を使う
- entity ハンドルを長期間保存する場合は世代を検証する
- 差分描画は正しさを全画面描画で確立した後に最適化として導入する
- 端末依存コードを `clione_terminal` 以外へ増やさない

## 最初に実装する最小スライス

最初の動作確認では、次の範囲だけを完成させる。

1. `core` の `World`、`Entity`、component storage、query、schedule
2. `terminal` の session 開始/終了と writer
3. `input` の矢印キー、`q`、リサイズと input system
4. `render` の全画面 ASCII 描画と render system
5. `clione` の fixed timestep + ECS schedule ループ
6. `Position` と `Velocity` を持つ `@` を system で動かす sandbox

この最小スライスが動いてから、色・レイヤー・差分描画・高度なキー解析・query キャッシュを追加する。これにより、早い段階で「入力イベントが component/resource を変え、system が entity を更新し、端末上の ASCII が更新され、終了時に端末が復元される」という主要な利用シナリオを検証できる。

## 現在の ECS API とゲームループ

component の参照方法で query の性質を指定する。

```rust
let read = world.query::<(&Position, &Velocity, &Renderable)>();
let mut write = world.query_mut::<(&mut Position, &Velocity)>();
```

`query` は任意の component 数（現在は 1〜8 要素のタプル）を読み取り、`query_mut` は `&T` と `&mut T` を混在させて取得する。同じ component 型を複数回 mutable borrow する query は拒否する。将来は query parameter の derive と動的 query を追加する。

利用者は `App` に `World` と `Schedule` をまとめて登録し、`App::update` または `App::run` でゲームループを実行できる。`App::run` は `AppExit` resource が終了を要求するまで tick を繰り返す。公開facadeでは `clione::prelude::*` から主要なECS・schedule APIをまとめて取り込める。

通常の stage system の実行順を明示する場合は、`App::add_system` が返す `SystemId` を `App::add_dependency` に渡す。Startup や OnEnter/OnExit は `App::add_systems` で登録する。

機能単位の再利用には `Plugin` trait を実装し、`App::add_plugins` で登録する。Plugin の `build` 内で resource、system、state、依存関係をまとめて構成できる。

複数の Query や Resource を一つのsystemで扱う場合は、`world.system_context()` を使用する。`SystemContext` はアクセス対象を記録し、異なるcomponent/resourceへの可変アクセスを同時に生成する。同一対象への競合アクセスは生成時に検出する。

SystemParam形式のsystemは `add_param_system` で登録できる。既存の `FnMut(&mut World)` systemも引き続き利用できる。

```rust
app.add_param_system(ScheduleStage::Update, |params: &mut SystemContext| {
    let score = params.resource_mut::<Score>().unwrap();
    let mut query = params.query_mut::<(&mut Position, &Velocity)>();
    // scoreとqueryを同じsystem内で利用する
});
```

`App::update` は実時間との差分を `Time` に反映し、`VirtualTime` の速度・一時停止を適用する。`FixedTime` の蓄積値が timestep を超えた回数だけ `FixedUpdate` を実行する。テストやリプレイでは `App::update_with_delta` で経過時間を明示できる。

構造変更は `World::commands()` に積み、system単位で自動flushする。`World::spawn_deferred` はEntityを予約してからflush時に有効化する。Eventは `OnEvent::<E>::new()` を条件に登録でき、App tick終了時にqueueがクリアされる。

アプリケーション開始時は `Schedule::run_startup` が `Startup` system を一度だけ実行する。状態リソースを `State<S>` として登録すると、`add_systems(OnEnter(value), system)` と `add_systems(OnExit(value), system)` で画面やゲーム状態の遷移処理を登録できる。通常の 1 tick は `ScheduleStage` の順番で system を実行する。通常の `System` は `&mut World` を受け取り直列実行し、`ReadOnlySystem` は `Schedule::run_parallel` で stage 内の他の read-only system と並列実行する。

```text
Startup
  -> OnEnter
  -> 入力収集
  -> PreInput
  -> Input
  -> Update
  -> PostUpdate / Commands 適用
  -> RenderPrepare
  -> Render
  -> 次の tick まで待機
```

`examples/sandbox` は現在、`Position`・`Velocity`・`Player` component を使い、固定 100ms tick でプレイヤーを移動して座標を表示する最小実行例になっている。端末入力と ANSI 描画は `clione_input`、`clione_terminal`、`clione_render` の実装後にこの schedule へ接続する。
