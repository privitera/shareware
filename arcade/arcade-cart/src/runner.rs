//! Standalone eframe shells for running a single cart or the full console
//! outside of any embedding host.
//!
//! Feature-gated behind `runner` (on by default). Disable with
//! `default-features = false` when embedding `arcade-cart` in another
//! egui app that runs its own event loop.
//!
//! ## Keyboard mapping
//!
//! - `A` / `D`: decrement / increment `rotation_left`
//! - `←` / `→`: decrement / increment `rotation_right`
//! - `rotation` is the sum of left + right
//! - `Space`: `action_pressed`
//! - `Esc`: `menu_requested` (return to menu, or exit single-game)
//! - `Q`: `exit_requested` (triggers CRT shutdown in console mode)

use std::time::Instant;

use egui::{Context, Key, ViewportBuilder, ViewportCommand};

use crate::cart::{Game, Input};
use crate::config::ConsoleConfig;
use crate::console::Console;

/// Radians per second when a rotation key is held.
const ROTATION_SPEED: f32 = 3.0;

/// Default standalone window size — portrait, half-scale of the 1080×1920
/// design reference, fits comfortably on common landscape monitors.
const DEFAULT_WINDOW: [f32; 2] = [540.0, 960.0];

fn read_input(ctx: &Context, dt: f32) -> Input {
    let mut input = Input::default();
    ctx.input(|i| {
        if i.key_down(Key::A) {
            input.rotation_left -= ROTATION_SPEED * dt;
        }
        if i.key_down(Key::D) {
            input.rotation_left += ROTATION_SPEED * dt;
        }
        if i.key_down(Key::ArrowLeft) {
            input.rotation_right -= ROTATION_SPEED * dt;
        }
        if i.key_down(Key::ArrowRight) {
            input.rotation_right += ROTATION_SPEED * dt;
        }
        input.rotation = input.rotation_left + input.rotation_right;
        input.action_pressed = i.key_pressed(Key::Space);
        input.menu_requested = i.key_pressed(Key::Escape);
        input.exit_requested = i.key_pressed(Key::Q);
    });
    input
}

struct GameApp<G: Game> {
    game: G,
    last_frame: Instant,
}

impl<G: Game> eframe::App for GameApp<G> {
    fn logic(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        let input = read_input(ctx, dt);
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

/// Open a window and run a single cart. Blocks until the window is closed
/// or the cart's [`Game::update`] returns `true`.
///
/// # Errors
///
/// Returns whatever [`eframe::run_native`] returns (mostly windowing /
/// graphics-context failures).
pub fn run_single<G: Game + 'static>(game: G) -> eframe::Result<()> {
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
                last_frame: Instant::now(),
            }) as Box<dyn eframe::App>)
        }),
    )
}

/// Open a window and run the full console (menu + carts + CRT shutdown).
///
/// # Errors
///
/// Returns whatever [`eframe::run_native`] returns.
pub fn run_console(games: Vec<Box<dyn Game>>, config: ConsoleConfig) -> eframe::Result<()> {
    run_single(Console::new(games, config))
}
