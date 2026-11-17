use crate::KeyboardInput;
use clione_core::KeyCode;

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
