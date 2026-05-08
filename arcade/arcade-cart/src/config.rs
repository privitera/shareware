//! Customizable text shown on the console home screen.
//!
//! All strings on the home screen are configurable so a host (e.g. an embedded
//! console) can re-skin the public defaults. Game *titles* are sourced from
//! [`crate::Game::name`] on each cart, not from this config.
//!
//! # TODO
//!
//! Future versions will expose a richer customization API: theme palette
//! overrides, font selection, layout/sizing knobs, optional banner art, and
//! per-game splash sprites. For now only the home-screen text is configurable.
//! Track the roadmap in `shareware/arcade/README.md`.

/// Text strings shown on the home screen / launcher.
#[derive(Clone, Debug)]
pub struct ConsoleConfig {
    /// Big banner text at the top (e.g. `"SHAREWARE"`, `"IMPULSE"`).
    pub title: String,

    /// Smaller banner under the title (e.g. `"A  R  C  A  D  E"`).
    pub subtitle: String,

    /// Blinking call-to-action above the controls strip (e.g.
    /// `">>> PRESS TO PLAY <<<"`).
    pub prompt: String,

    /// First line of the controls strip near the bottom.
    pub controls_select: String,

    /// Second line of the controls strip.
    pub controls_start: String,

    /// Final hint at the very bottom (e.g. `"ESC TO EXIT"`).
    pub exit_hint: String,
}

impl Default for ConsoleConfig {
    /// Public/standalone defaults — neutral text, no host-specific gestures.
    fn default() -> Self {
        Self {
            title: "SHAREWARE".to_string(),
            subtitle: "A  R  C  A  D  E".to_string(),
            prompt: ">>> PRESS TO PLAY <<<".to_string(),
            controls_select: "ROTATE = SELECT".to_string(),
            controls_start: "PRESS = START".to_string(),
            exit_hint: "ESC TO EXIT".to_string(),
        }
    }
}
