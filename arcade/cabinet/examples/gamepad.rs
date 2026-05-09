//! Example: drive the shareware arcade with a gamepad via [`gilrs`].
//!
//! Run with: `cargo run --example gamepad -p cabinet`
//!
//! Demonstrates the [`arcade_cart::InputSource`] trait. The same pattern
//! works for any input device — USB rotary encoders, real arcade controls,
//! a scripted replay harness, or a hybrid that combines several sources.
//!
//! ## Mapping
//!
//! | Source                  | Cart input              |
//! |-------------------------|-------------------------|
//! | Left stick X            | `rotation_left`         |
//! | Right stick X           | `rotation_right`        |
//! | West button (X / □)     | `action_pressed_left`   |
//! | East button (B / ○)     | `action_pressed_right`  |
//! | South button (A / ×)    | `action_pressed` only   |
//! | D-pad down              | `orient_down` (skifree's "face downhill") |
//! | Select / Back           | `menu_requested`        |
//! | Start / Mode            | `exit_requested`        |
//!
//! Stick rotation is integrated as `stick_value × ROTATION_SPEED × dt` so
//! the further you push, the faster the snake/skier turns — matching the
//! "knob held at angle" feel.

use arcade_cart::{ConsoleConfig, Game, Input, InputSource};
use gilrs::{Axis, Button, EventType, Gilrs};

const STICK_DEADZONE: f32 = 0.15;
const ROTATION_SPEED: f32 = 3.0;

struct GamepadInput {
    gilrs: Gilrs,
}

impl GamepadInput {
    fn new() -> Self {
        Self {
            gilrs: Gilrs::new().expect("failed to init gilrs (no gamepad subsystem?)"),
        }
    }
}

impl InputSource for GamepadInput {
    fn poll(&mut self, _ctx: &egui::Context, dt: f32) -> Input {
        let mut input = Input::default();

        // Drain rising-edge button events — `is_pressed` would return held
        // state, which would re-fire restart/menu actions every frame.
        let mut south_pressed = false;
        let mut west_pressed = false;
        let mut east_pressed = false;
        let mut select_pressed = false;
        let mut start_pressed = false;
        let mut dpad_down_pressed = false;
        while let Some(gilrs::Event { event, .. }) = self.gilrs.next_event() {
            if let EventType::ButtonPressed(button, _) = event {
                match button {
                    Button::South => south_pressed = true,
                    Button::West => west_pressed = true,
                    Button::East => east_pressed = true,
                    Button::DPadDown => dpad_down_pressed = true,
                    Button::Select => select_pressed = true,
                    Button::Start | Button::Mode => start_pressed = true,
                    _ => {}
                }
            }
        }

        // Continuous: stick deflection → rotation delta.
        if let Some((_, gamepad)) = self.gilrs.gamepads().next() {
            let lx = gamepad.value(Axis::LeftStickX);
            let rx = gamepad.value(Axis::RightStickX);
            if lx.abs() > STICK_DEADZONE {
                input.rotation_left = lx * ROTATION_SPEED * dt;
            }
            if rx.abs() > STICK_DEADZONE {
                input.rotation_right = rx * ROTATION_SPEED * dt;
            }
            input.rotation = input.rotation_left + input.rotation_right;
        }

        input.action_pressed_left = west_pressed;
        input.action_pressed_right = east_pressed;
        input.action_pressed = south_pressed || west_pressed || east_pressed;
        input.orient_down = dpad_down_pressed;
        input.menu_requested = select_pressed;
        input.exit_requested = start_pressed;

        input
    }
}

fn main() {
    let games: Vec<Box<dyn Game>> = vec![
        Box::new(skifree::SkiFree::new()),
        Box::new(pong::Pong::new()),
        Box::new(breakout::Breakout::new()),
        Box::new(snake::SnakeGame::new()),
        Box::new(etch::Sketch::new()),
    ];

    if let Err(err) = arcade_cart::runner::run_console_with(
        games,
        ConsoleConfig::default(),
        GamepadInput::new(),
    ) {
        eprintln!("gamepad: failed to start window: {err}");
        std::process::exit(1);
    }
}
