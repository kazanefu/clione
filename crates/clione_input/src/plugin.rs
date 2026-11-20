use clione_core::{App, InputEvent, Plugin, ScheduleStage, SystemContext};

use crate::{keyboard::KeyboardInput, text::TextInput};

/// A plugin that adds input handling to the application.
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(KeyboardInput::default());
        app.insert_resource(TextInput::default());

        // Register the input message queue.
        app.world_mut().add_messages::<InputEvent>();

        // Clear transient states at the beginning of each frame.
        app.add_param_system(ScheduleStage::PreInput, |ctx: &mut SystemContext<'_>| {
            if let Some(input) = ctx.resource_mut::<KeyboardInput>() {
                input.clear_frame();
            }
        });
        app.add_param_system(ScheduleStage::PreInput, |ctx: &mut SystemContext<'_>| {
            if let Some(input) = ctx.resource_mut::<TextInput>() {
                input.clear_frame();
            }
        });

        // Apply input events to KeyboardInput.
        app.add_param_system(ScheduleStage::Input, |ctx: &mut SystemContext<'_>| {
            let Some(mut reader) = ctx.message_reader::<InputEvent>() else {
                return;
            };

            let Some(input) = ctx.resource_mut::<KeyboardInput>() else {
                return;
            };

            for event in reader.read() {
                input.process_event(event);
            }
        });
        // Apply input events to TextInput.
        app.add_param_system(ScheduleStage::Input, |ctx: &mut SystemContext<'_>| {
            let Some(mut reader) = ctx.message_reader::<InputEvent>() else {
                return;
            };

            let Some(input) = ctx.resource_mut::<TextInput>() else {
                return;
            };

            for event in reader.read() {
                input.process_event(event);
            }
        });
    }
}
