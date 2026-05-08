//! Cartridge contract: the [`Game`] trait, [`Input`] state, and a small [`Rng`].
//!
//! These three types are the entire cartridge surface. Anything that implements
//! [`Game`] can run on the [`crate::Console`] or via [`crate::runner`].

use egui::Ui;

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

    /// Primary action button was pressed this frame (rising edge).
    pub action_pressed: bool,

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
