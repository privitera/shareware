//! Standalone eframe shells for running a single cart or the full console
//! outside of any embedding host.
//!
//! Feature-gated behind `runner` (on by default). Disable with
//! `default-features = false` when embedding `arcade-cart` in another
//! egui app that runs its own event loop.
//!
//! ## Default keyboard mapping
//!
//! See [`crate::KeyboardInput`] for the full table.
//!
//! ## Custom input sources
//!
//! [`run_single_with`] / [`run_console_with`] take any
//! [`crate::InputSource`] — gamepad, USB rotary, scripted replay,
//! whatever. The plain [`run_single`] / [`run_console`] use
//! [`crate::KeyboardInput`] by default.

use std::time::Instant;

use egui::{Context, ViewportBuilder, ViewportCommand};

use crate::cart::{Game, InputSource, KeyboardInput};
use crate::config::ConsoleConfig;
use crate::console::Console;

/// Default standalone window size — portrait, half-scale of the 1080×1920
/// design reference, fits comfortably on common landscape monitors.
const DEFAULT_WINDOW: [f32; 2] = [540.0, 960.0];

struct GameApp<G: Game, I: InputSource> {
    game: G,
    input: I,
    last_frame: Instant,
}

impl<G: Game, I: InputSource> eframe::App for GameApp<G, I> {
    fn logic(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        let input = self.input.poll(ctx, dt);
        let done = self.game.update(dt, &input);

        if done {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }

        ctx.request_repaint();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);
        self.game.render(ui);
    }
}

/// Open a window and run a single cart with the default keyboard input.
///
/// # Errors
///
/// Returns whatever [`eframe::run_native`] returns (mostly windowing /
/// graphics-context failures).
pub fn run_single<G: Game + 'static>(game: G) -> eframe::Result<()> {
    run_single_with(game, KeyboardInput::default())
}

/// Open a window and run a single cart with a caller-supplied input source.
///
/// Use this to plug in a gamepad, real arcade controls, a USB rotary, a
/// replay harness, or any combination thereof.
///
/// # Errors
///
/// Returns whatever [`eframe::run_native`] returns.
pub fn run_single_with<G, I>(game: G, input: I) -> eframe::Result<()>
where
    G: Game + 'static,
    I: InputSource + 'static,
{
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size(DEFAULT_WINDOW)
            .with_title("Shareware Arcade"),
        ..Default::default()
    };
    eframe::run_native(
        "Shareware Arcade",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(GameApp {
                game,
                input,
                last_frame: Instant::now(),
            }) as Box<dyn eframe::App>)
        }),
    )
}

/// Open a window and run the full console (menu + carts + CRT shutdown)
/// with the default keyboard input.
///
/// # Errors
///
/// Returns whatever [`eframe::run_native`] returns.
pub fn run_console(games: Vec<Box<dyn Game>>, config: ConsoleConfig) -> eframe::Result<()> {
    run_console_with(games, config, KeyboardInput::default())
}

/// Open a window and run the full console with a caller-supplied input
/// source.
///
/// # Errors
///
/// Returns whatever [`eframe::run_native`] returns.
pub fn run_console_with<I>(
    games: Vec<Box<dyn Game>>,
    config: ConsoleConfig,
    input: I,
) -> eframe::Result<()>
where
    I: InputSource + 'static,
{
    run_single_with(Console::new(games, config), input)
}
