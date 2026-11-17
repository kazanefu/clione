use clione_core::{App, InputEvent, InputState, KeyCode};

use crate::{InputPlugin, KeyboardInput};

#[test]
fn key_press() {
    let mut input = KeyboardInput::default();

    input.press(KeyCode::KeyW);

    assert!(input.pressed(KeyCode::KeyW));
    assert!(input.just_pressed(KeyCode::KeyW));
    assert!(!input.just_released(KeyCode::KeyW));
}

#[test]
fn key_release() {
    let mut input = KeyboardInput::default();

    input.press(KeyCode::KeyW);
    input.clear_frame();
    input.release(KeyCode::KeyW);

    assert!(!input.pressed(KeyCode::KeyW));
    assert!(!input.just_pressed(KeyCode::KeyW));
    assert!(input.just_released(KeyCode::KeyW));
}

#[test]
fn repeated_press() {
    let mut input = KeyboardInput::default();

    input.press(KeyCode::KeyW);
    input.press(KeyCode::KeyW);

    assert!(input.just_pressed(KeyCode::KeyW));
}

#[test]
fn keyboard_input_plugin() {
    let mut app = App::new();
    app.add_plugins(InputPlugin);

    app.world_mut()
        .send_message(InputEvent::KeyDown(KeyCode::KeyW));

    app.update();

    let input = app.world().resource::<KeyboardInput>().unwrap();

    assert_eq!(input.get(KeyCode::KeyW), InputState::JustPressed);
    assert!(input.pressed(KeyCode::KeyW));
    assert!(input.just_pressed(KeyCode::KeyW));

    // Next frame: transient state is cleared.
    app.update();

    let input = app.world().resource::<KeyboardInput>().unwrap();

    assert_eq!(input.get(KeyCode::KeyW), InputState::Pressed);
    assert!(!input.just_pressed(KeyCode::KeyW));

    // Release the key.
    app.world_mut()
        .send_message(InputEvent::KeyUp(KeyCode::KeyW));

    app.update();

    let input = app.world().resource::<KeyboardInput>().unwrap();

    assert_eq!(input.get(KeyCode::KeyW), InputState::JustReleased);
    assert!(!input.pressed(KeyCode::KeyW));
}
