use clione_core::{App, InputEvent, InputState, Key, KeyCode};

use crate::{InputPlugin, KeyboardInput};

#[test]
fn key_press() {
    let mut input = KeyboardInput::default();

    input.press_code(Some(KeyCode::KeyW));

    assert!(input.pressed(KeyCode::KeyW));
    assert!(input.just_pressed(KeyCode::KeyW));
    assert!(!input.just_released(KeyCode::KeyW));
}

#[test]
fn key_release() {
    let mut input = KeyboardInput::default();

    input.press_code(Some(KeyCode::KeyW));
    input.clear_frame();
    input.release_code(Some(KeyCode::KeyW));

    assert!(!input.pressed(KeyCode::KeyW));
    assert!(!input.just_pressed(KeyCode::KeyW));
    assert!(input.just_released(KeyCode::KeyW));
}

#[test]
fn repeated_press() {
    let mut input = KeyboardInput::default();

    input.press_code(Some(KeyCode::KeyW));
    input.press_code(Some(KeyCode::KeyW));

    assert!(input.just_pressed(KeyCode::KeyW));
}

#[test]
fn keyboard_input_plugin() {
    let mut app = App::new();
    app.add_plugins(InputPlugin);

    app.world_mut()
        .send_message(InputEvent::Keyboard(clione_core::KeyboardEvent {
            key: clione_core::Key::Char('w'),
            key_code: Some(KeyCode::KeyW),
            state: clione_core::ButtonState::Pressed,
        }));

    app.update();

    let input = app.world().resource::<KeyboardInput>().unwrap();

    assert_eq!(input.get(KeyCode::KeyW), InputState::JustPressed);
    assert!(input.pressed(KeyCode::KeyW));
    assert!(input.just_pressed(KeyCode::KeyW));
    assert!(input.just_pressed(Key::Char('w')));

    // Next frame: transient state is cleared.
    app.update();

    let input = app.world().resource::<KeyboardInput>().unwrap();

    assert_eq!(input.get(KeyCode::KeyW), InputState::Pressed);
    assert!(!input.just_pressed(KeyCode::KeyW));
    assert!(!input.just_pressed(Key::Char('w')));

    // Release the key.
    app.world_mut()
        .send_message(InputEvent::Keyboard(clione_core::KeyboardEvent {
            key: clione_core::Key::Char('w'),
            key_code: Some(KeyCode::KeyW),
            state: clione_core::ButtonState::Released,
        }));

    app.update();

    let input = app.world().resource::<KeyboardInput>().unwrap();

    assert_eq!(input.get(KeyCode::KeyW), InputState::JustReleased);
    assert!(!input.pressed(KeyCode::KeyW));
}
