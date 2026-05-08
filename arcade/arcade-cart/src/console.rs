//! [`Console`]: home screen + game lifecycle + CRT shutdown overlay.
//!
//! Owns a `Vec<Box<dyn Game>>` of cartridges and routes input to either the
//! menu or the currently-playing cart. When the host signals
//! `input.exit_requested`, the CRT shutdown plays before [`Game::update`]
//! returns `true`.

use egui::Ui;

use crate::cart::{Game, Input};
use crate::config::ConsoleConfig;
use crate::menu::Menu;
use crate::shutdown::Shutdown;

enum State {
    Menu(Menu),
    Playing(usize),
}

/// The arcade console. Implements [`Game`] so it can be hosted by
/// [`crate::runner::run_single`] or embedded inside another `Game`.
pub struct Console {
    games: Vec<Box<dyn Game>>,
    config: ConsoleConfig,
    state: State,
    shutdown: Shutdown,
}

impl Console {
    /// Build a new console.
    ///
    /// The `games` order determines menu order. Menu starts with the first
    /// cart selected. `config` controls the home-screen text; pass
    /// [`ConsoleConfig::default`] for the public/standalone defaults.
    #[must_use]
    pub fn new(games: Vec<Box<dyn Game>>, config: ConsoleConfig) -> Self {
        Self {
            games,
            config,
            state: State::Menu(Menu::new()),
            shutdown: Shutdown::new(),
        }
    }
}

impl Game for Console {
    fn update(&mut self, dt: f32, input: &Input) -> bool {
        if self.shutdown.is_active() {
            return self.shutdown.update(dt);
        }

        if input.exit_requested {
            self.shutdown.start();
            return false;
        }

        match &mut self.state {
            State::Menu(menu) => {
                if let Some(idx) = menu.update(input, self.games.len()) {
                    self.state = State::Playing(idx);
                }
            }
            State::Playing(idx) => {
                if input.menu_requested {
                    self.state = State::Menu(Menu::new());
                    return false;
                }
                let game_done = self.games[*idx].update(dt, input);
                if game_done {
                    self.state = State::Menu(Menu::new());
                }
            }
        }

        false
    }

    fn render(&self, ui: &mut Ui) {
        match &self.state {
            State::Menu(menu) => menu.render(ui, &self.games, &self.config),
            State::Playing(idx) => self.games[*idx].render(ui),
        }
        self.shutdown.render(ui);
    }

    fn name(&self) -> &str {
        match &self.state {
            State::Menu(_) => &self.config.title,
            State::Playing(idx) => self.games[*idx].name(),
        }
    }

    fn debug_stats(&self) -> Option<String> {
        match &self.state {
            State::Menu(_) => Some("state=menu".into()),
            State::Playing(idx) => self.games[*idx].debug_stats(),
        }
    }
}
