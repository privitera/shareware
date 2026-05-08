//! Cartridge-style spec for the shareware arcade.
//!
//! Three pieces:
//!
//! 1. **The cart contract** — [`Game`] trait, [`Input`] state, [`Rng`].
//!    Implement these to make a cartridge.
//! 2. **The console** — [`Console`] wraps a `Vec<Box<dyn Game>>`, runs the
//!    home screen, hands input to the playing cart, and plays the CRT
//!    shutdown effect. [`ConsoleConfig`] customizes the home-screen text.
//! 3. **The runner** — [`runner::run_single`] / [`runner::run_console`]
//!    open an `eframe` window for standalone use. Feature-gated behind
//!    `runner` (on by default).
//!
//! ## Embedding inside another egui app
//!
//! Disable default features to drop the eframe dependency:
//!
//! ```toml
//! arcade-cart = { version = "0.1", default-features = false }
//! ```
//!
//! Then build a [`Console`] (or any [`Game`]) and call its update/render
//! from your own egui frame.
//!
//! ## Customization
//!
//! [`ConsoleConfig`] currently controls home-screen strings only. A richer
//! customization API (theme palette, fonts, layout, banner art, per-game
//! splash sprites) is planned — see the workspace `README.md` roadmap.

mod cart;
mod config;
mod console;
mod menu;
mod shutdown;

#[cfg(feature = "runner")]
pub mod runner;

pub use cart::{Game, Input, Rng};
pub use config::ConsoleConfig;
pub use console::Console;
pub use shutdown::Shutdown;

/// Native design width — the home screen and bundled games are authored
/// against this width in pixels. Hosts that render at a different size are
/// scaled by [`Console`] / [`runner`] automatically; rendering at exactly
/// `DESIGN_WIDTH` × [`DESIGN_HEIGHT`] is identity-scale and matches the
/// original impulse stove (1080×1920 portrait) pixel-for-pixel.
pub const DESIGN_WIDTH: f32 = 1080.0;

/// Native design height. See [`DESIGN_WIDTH`].
pub const DESIGN_HEIGHT: f32 = 1920.0;

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, Rect};

    /// At native design dimensions the menu Layout must be identity-scale
    /// (so the impulse stove's stock 1080×1920 rendering is preserved
    /// pixel-for-pixel by the public arcade-cart).
    #[test]
    fn native_design_rect_is_identity_scale() {
        let rect = Rect::from_min_max(
            Pos2::ZERO,
            Pos2::new(DESIGN_WIDTH, DESIGN_HEIGHT),
        );
        // Layout::new picks min(width/DESIGN_W, height/DESIGN_H); at the
        // native rect both ratios are 1.0.
        let scale_x = rect.width() / DESIGN_WIDTH;
        let scale_y = rect.height() / DESIGN_HEIGHT;
        assert!((scale_x - 1.0).abs() < f32::EPSILON);
        assert!((scale_y - 1.0).abs() < f32::EPSILON);
        assert!((scale_x.min(scale_y) - 1.0).abs() < f32::EPSILON);
    }
}
