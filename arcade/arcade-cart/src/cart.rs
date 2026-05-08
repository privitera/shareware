//! Cartridge contract: the [`Game`] trait, [`Input`] state, and a small [`Rng`].
//!
//! These three types are the entire cartridge surface. Anything that implements
//! [`Game`] can run on the [`crate::Console`] or via [`crate::runner`].

use egui::{Context, Key, Ui};

/// A cartridge: a game that runs on the arcade console.
///
/// Implementations receive a fixed-rate update tick and an immutable render pass
/// against an [`egui::Ui`]. The host (console or standalone runner) supplies an
/// [`Input`] each frame.
pub trait Game: Send + Sync {
    /// Step the simulation by `dt` seconds.
    ///
    /// Return `true` to signal "I'm done, return to menu / exit". Return `false`
    /// to keep playing.
    fn update(&mut self, dt: f32, input: &Input) -> bool;

    /// Paint the current frame into `ui`.
    fn render(&self, ui: &mut Ui);

    /// Display name shown on the home screen (e.g. `"PONG"`).
    fn name(&self) -> &str;

    /// Optional one-line tagline shown under the name on the home screen
    /// (e.g. `"WATCH OUT FOR TREES"`). Default: empty.
    fn description(&self) -> &str {
        ""
    }

    /// Optional year of the original this cart pays homage to (e.g. `"1991"`).
    /// Rendered next to the description. Default: empty.
    fn year(&self) -> &str {
        ""
    }

    /// Optional debug stats (e.g. entity counts, FPS, memory). Default: `None`.
    fn debug_stats(&self) -> Option<String> {
        None
    }
}

/// Input state from the host, accumulated over the current frame.
///
/// Rotation fields are in radians and represent *delta* this frame, not
/// absolute angle. Hosts wire whatever they have:
/// - rotary encoders / knobs → use `rotation`, optionally split L/R
/// - keyboard → map keys to small per-frame deltas (the runner does this)
/// - gamepad sticks → integrate stick value × dt
#[derive(Debug, Clone, Default)]
pub struct Input {
    /// Combined rotation delta from all input sources this frame. Used by
    /// single-player games that don't care about input partitioning.
    pub rotation: f32,

    /// Rotation delta from the "left" half of the input. Used by 2P games
    /// (left paddle / left player) and split-axis games like sketch (X axis).
    pub rotation_left: f32,

    /// Rotation delta from the "right" half of the input. Used by 2P games
    /// (right paddle / right player) and split-axis games like sketch (Y axis).
    pub rotation_right: f32,

    /// Primary action button was pressed this frame (rising edge). This is
    /// the OR of every per-player button — single-button hosts (or any-button
    /// designs) can ignore the per-player fields and only set this one.
    pub action_pressed: bool,

    /// Per-player action button — left side (P1). For multi-encoder hosts,
    /// this is the OR of all clicks on the left half. Cartridges that are
    /// 2P-aware (e.g. fighting moves, per-player boost) read this; 1P-only
    /// games can ignore it. Default `false`.
    pub action_pressed_left: bool,

    /// Per-player action button — right side (P2). See [`action_pressed_left`].
    pub action_pressed_right: bool,

    /// Host requests the cart return to the home menu (e.g. game-over hotkey).
    pub menu_requested: bool,

    /// Host requests the console exit entirely (e.g. window close).
    pub exit_requested: bool,
}

/// Tiny deterministic XorShift PRNG.
///
/// Not cryptographically anything. Suitable for arcade randomness: spawn
/// positions, sprite jitter, twinkle phase. Deterministic from the seed.
pub struct Rng(u32);

impl Rng {
    /// Create a new RNG. A seed of `0` is replaced with a default so the
    /// generator never gets stuck.
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self(if seed == 0 { 0x1234_5678 } else { seed })
    }

    /// Next raw u32.
    pub fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    /// Random u32 in `[0, max)`. Panics-free when `max == 0` (returns `0`).
    pub fn range(&mut self, max: u32) -> u32 {
        if max == 0 {
            0
        } else {
            self.next() % max
        }
    }

    /// Random f32 in `[0.0, 1.0)`.
    pub fn f32(&mut self) -> f32 {
        (self.next() % 10_000) as f32 / 10_000.0
    }
}

/// Pluggable input source for the standalone runner.
///
/// Implement this to wire any input device (keyboard, gamepad, USB rotary
/// encoder, real arcade controls, replay/test harness, ...) into the
/// console. The runner calls [`poll`](Self::poll) once per frame.
///
/// The default implementation is [`KeyboardInput`]; the `runner` module
/// uses it when callers don't supply their own.
///
/// Only `Send` is required (not `Sync`) so input sources holding
/// non-`Sync` types — e.g. `gilrs::Gilrs` (mpsc receiver) — fit the trait
/// directly. eframe is single-threaded, so the runner never shares an
/// `InputSource` across threads.
pub trait InputSource: Send {
    /// Read the input source for the current frame.
    ///
    /// `dt` is the wall-clock time since the previous poll (clamped). Use
    /// it to integrate continuous inputs (held keys, stick deflection) into
    /// per-frame rotation deltas.
    fn poll(&mut self, ctx: &Context, dt: f32) -> Input;
}

/// Default keyboard input source.
///
/// Each "side" of the keyboard simulates a rotary encoder:
///
/// | Key       | Effect                             |
/// |-----------|------------------------------------|
/// | `A` / `D` | `rotation_left` − / +              |
/// | `←` / `→` | `rotation_right` − / +             |
/// | `Tab`     | `action_pressed_left` (P1 click)   |
/// | `Enter`   | `action_pressed_right` (P2 click)  |
/// | `Space`   | `action_pressed` only (shared/1P)  |
/// | `Esc`     | `menu_requested`                   |
/// | `Q`       | `exit_requested`                   |
///
/// `action_pressed` is the OR of `Space`, `Tab`, and `Enter` — 1P games
/// reading only `action_pressed` accept any of those keys. egui doesn't
/// expose left/right `Shift` separately, so `Tab` and `Enter` were chosen
/// as geographically left/right action buttons that egui *does* expose.
pub struct KeyboardInput {
    /// Radians per second of rotation while a rotation key is held. Default 3.0.
    pub rotation_speed: f32,
}

impl Default for KeyboardInput {
    fn default() -> Self {
        Self { rotation_speed: 3.0 }
    }
}

impl InputSource for KeyboardInput {
    fn poll(&mut self, ctx: &Context, dt: f32) -> Input {
        let mut input = Input::default();
        ctx.input(|i| {
            if i.key_down(Key::A) {
                input.rotation_left -= self.rotation_speed * dt;
            }
            if i.key_down(Key::D) {
                input.rotation_left += self.rotation_speed * dt;
            }
            if i.key_down(Key::ArrowLeft) {
                input.rotation_right -= self.rotation_speed * dt;
            }
            if i.key_down(Key::ArrowRight) {
                input.rotation_right += self.rotation_speed * dt;
            }
            input.rotation = input.rotation_left + input.rotation_right;

            input.action_pressed_left = i.key_pressed(Key::Tab);
            input.action_pressed_right = i.key_pressed(Key::Enter);
            input.action_pressed = i.key_pressed(Key::Space)
                || input.action_pressed_left
                || input.action_pressed_right;

            input.menu_requested = i.key_pressed(Key::Escape);
            input.exit_requested = i.key_pressed(Key::Q);
        });
        input
    }
}
