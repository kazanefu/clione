//! Public facade for the Clione terminal game engine.

pub use clione_core;
pub use clione_core::{
    Added, App, AppExit, ButtonState, Cell, Changed, Color, Commands, Entity, Events, FixedTime,
    Frame, InputEvent, InputState, IntoScheduleTarget, Key, KeyCode, KeyboardEvent, Layer,
    MessageReader, MessageWriter, Messages, OnEnter, OnEvent, OnExit, OnMessage, ParamSystem,
    Plugin, Point, Query, Query2, QueryMut, QueryMutIter, QueryMutSpec, QuerySpec, ReadOnlySystem,
    Rect, Schedule, ScheduleError, ScheduleStage, ScheduleTarget, Size, State, System,
    SystemContext, SystemId, TerminalSize, Time, VirtualTime, With, Without, World, WorldError,
    init_state,
};
pub use clione_input;
pub use clione_input::{
    InputMode, InputPlugin, InputSettings, KeyCodeSupport, KeyIdentifier, KeyboardInput,
    TerminalCapabilities, TextInput,
};

/// Commonly used Clione types.
pub mod prelude {
    pub use crate::{
        Added, App, AppExit, ButtonState, Cell, Changed, Color, Commands, Entity, Events,
        FixedTime, Frame, InputEvent, InputMode, InputPlugin, InputSettings, InputState,
        IntoScheduleTarget, Key, KeyCode, KeyCodeSupport, KeyboardEvent, KeyboardInput, Layer,
        MessageReader, MessageWriter, Messages, OnEnter, OnEvent, OnExit, OnMessage, ParamSystem,
        Plugin, Point, Query, Query2, QueryMut, QueryMutIter, QueryMutSpec, QuerySpec,
        ReadOnlySystem, Rect, Schedule, ScheduleError, ScheduleStage, Size, State, System,
        SystemContext, SystemId, TerminalCapabilities, TerminalSize, TextInput, Time, VirtualTime,
        With, Without, World, WorldError, init_state,
    };
}
