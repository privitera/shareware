//! Home screen / launcher.
//!
//! Retro arcade cabinet aesthetic: starfield, perspective grid, neon brackets,
//! glitching title, blinking call-to-action, scanline overlay, hum bars,
//! rolling interference, and CRT vignette.
//!
//! Generic over the cart list — game names/descriptions/years come from
//! [`crate::Game`] trait methods on each cart in the `games` slice.
//! Title/subtitle/footer text come from [`ConsoleConfig`].

use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};

use crate::cart::{Game, Input, Rng};
use crate::config::ConsoleConfig;

// === NEON ARCADE COLOR PALETTE ===
const NEON_CYAN: Color32 = Color32::from_rgb(0, 255, 255);
const NEON_MAGENTA: Color32 = Color32::from_rgb(255, 0, 255);
const NEON_YELLOW: Color32 = Color32::from_rgb(255, 255, 0);
const NEON_GREEN: Color32 = Color32::from_rgb(57, 255, 20);
const ARCADE_DARK: Color32 = Color32::from_rgb(8, 8, 16);
const GRID_COLOR: Color32 = Color32::from_rgb(40, 0, 80);
const SCANLINE_COLOR: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 50);

// Design coordinates were authored against the native target rect:
// 1080×1920 portrait, the impulse stove screen aspect. `Layout` scales them
// to whatever rect the host gives us; at the native size it's identity.
use crate::{DESIGN_HEIGHT as DESIGN_H, DESIGN_WIDTH as DESIGN_W};

struct Layout {
    rect: Rect,
    scale: f32,
}

impl Layout {
    fn new(rect: Rect) -> Self {
        let scale = (rect.height() / DESIGN_H).min(rect.width() / DESIGN_W);
        Self { rect, scale }
    }

    /// Map a design-space y offset (from top of rect) to screen y.
    fn y(&self, design_y: f32) -> f32 {
        self.rect.min.y + design_y * (self.rect.height() / DESIGN_H)
    }

    /// Map a design-space y offset measured from the bottom of rect.
    fn y_from_bottom(&self, design_y: f32) -> f32 {
        self.rect.max.y - design_y * (self.rect.height() / DESIGN_H)
    }

    /// Scale a design-space pixel value uniformly (for fonts, stroke widths).
    fn px(&self, design_px: f32) -> f32 {
        design_px * self.scale
    }
}

struct Star {
    x: f32, // normalized 0..1
    y: f32, // normalized 0..1
    speed: f32,
    brightness: u8,
}

struct HumBar {
    phase: f32,
    duration: f32,
    jitter_x: f32,
}

struct RollingInterference {
    phase: f32,
    duration: f32,
}

/// Home-screen state. Owned by [`crate::Console`].
pub struct Menu {
    selected_index: usize,
    rotation_accumulator: f32,
    frame_count: u32,
    time: f32,
    stars: Vec<Star>,
    hum_bars: Vec<HumBar>,
    rolling_interference: Vec<RollingInterference>,
    rng: Rng,
    glitch_timer: f32,
    glitch_slice_offset: f32,
    glitch_active: bool,
}

impl Menu {
    const ROTATION_THRESHOLD: f32 = 0.5;
    const NUM_STARS: usize = 100;

    #[must_use]
    pub fn new() -> Self {
        let mut rng = Rng::new(0xCAFE_BABE);
        let mut stars = Vec::with_capacity(Self::NUM_STARS);
        for _ in 0..Self::NUM_STARS {
            stars.push(Star {
                x: rng.f32(),
                y: rng.f32(),
                speed: 30.0 + (rng.next() % 120) as f32,
                brightness: 80 + (rng.next() % 176) as u8,
            });
        }

        Self {
            selected_index: 0,
            rotation_accumulator: 0.0,
            frame_count: 0,
            time: 0.0,
            stars,
            hum_bars: vec![
                HumBar { phase: 0.0, duration: 5.5, jitter_x: 0.0 },
                HumBar { phase: 0.33, duration: 5.8, jitter_x: 0.0 },
                HumBar { phase: 0.66, duration: 6.2, jitter_x: 0.0 },
            ],
            rolling_interference: vec![
                RollingInterference { phase: 0.0, duration: 15.0 },
                RollingInterference { phase: 0.5, duration: 14.0 },
            ],
            rng,
            glitch_timer: 0.0,
            glitch_slice_offset: 0.0,
            glitch_active: false,
        }
    }

    /// Advance one frame. If a game is selected this frame, returns its index
    /// in the host's game list (clamped to `game_count - 1`).
    pub fn update(&mut self, input: &Input, game_count: usize) -> Option<usize> {
        self.frame_count = self.frame_count.wrapping_add(1);
        let dt = 1.0 / 60.0;
        self.time += dt;

        self.update_glitch(dt);
        self.update_crt_effects(dt);

        // Starfield: scroll down at normalized speed, respawn at top.
        for star in &mut self.stars {
            star.y += star.speed * dt / DESIGN_H;
            if star.y > 1.0 {
                star.y = 0.0;
                star.x = self.rng.f32();
            }
        }

        // Menu nav by accumulated rotation.
        if game_count > 0 {
            self.rotation_accumulator += input.rotation;
            if self.rotation_accumulator > Self::ROTATION_THRESHOLD {
                self.rotation_accumulator = 0.0;
                if self.selected_index + 1 < game_count {
                    self.selected_index += 1;
                }
            } else if self.rotation_accumulator < -Self::ROTATION_THRESHOLD {
                self.rotation_accumulator = 0.0;
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            self.selected_index = self.selected_index.min(game_count.saturating_sub(1));

            if input.action_pressed {
                return Some(self.selected_index);
            }
        }

        None
    }

    fn update_glitch(&mut self, dt: f32) {
        if !self.glitch_active && self.rng.next() % 150 == 0 {
            self.glitch_active = true;
            self.glitch_timer = 0.1 + self.rng.f32() * 0.2;
        }

        if self.glitch_active {
            self.glitch_timer -= dt;
            self.glitch_slice_offset = (self.rng.f32() - 0.5) * 60.0;
            if self.glitch_timer <= 0.0 {
                self.glitch_active = false;
                self.glitch_slice_offset = 0.0;
            }
        }
    }

    fn update_crt_effects(&mut self, dt: f32) {
        for bar in &mut self.hum_bars {
            bar.phase += dt / bar.duration;
            if bar.phase >= 1.0 {
                bar.phase -= 1.0;
            }
            bar.jitter_x = match self.frame_count % 3 {
                0 => 0.5,
                1 => -0.5,
                _ => 0.0,
            };
        }

        for roll in &mut self.rolling_interference {
            roll.phase += dt / roll.duration;
            if roll.phase >= 1.0 {
                roll.phase -= 1.0;
            }
        }
    }

    fn draw_glitch_title(
        &self,
        painter: &Painter,
        center_x: f32,
        y: f32,
        text: &str,
        size: f32,
    ) {
        if self.glitch_active {
            painter.text(
                Pos2::new(center_x - 6.0, y - 2.0),
                Align2::CENTER_CENTER,
                text,
                FontId::monospace(size),
                Color32::from_rgba_unmultiplied(0, 255, 255, 200),
            );
            painter.text(
                Pos2::new(center_x + 6.0, y + 2.0),
                Align2::CENTER_CENTER,
                text,
                FontId::monospace(size),
                Color32::from_rgba_unmultiplied(255, 0, 255, 200),
            );
            painter.text(
                Pos2::new(center_x + self.glitch_slice_offset, y),
                Align2::CENTER_CENTER,
                text,
                FontId::monospace(size),
                Color32::from_rgba_unmultiplied(255, 255, 255, 220),
            );
        }

        painter.text(
            Pos2::new(center_x + 2.0, y + 2.0),
            Align2::CENTER_CENTER,
            text,
            FontId::monospace(size),
            Color32::from_rgba_unmultiplied(0, 80, 0, 100),
        );

        painter.text(
            Pos2::new(center_x, y),
            Align2::CENTER_CENTER,
            text,
            FontId::monospace(size),
            NEON_GREEN,
        );
    }

    /// Render the home screen.
    pub fn render(&self, ui: &mut Ui, games: &[Box<dyn Game>], config: &ConsoleConfig) {
        let rect = ui.available_rect_before_wrap();
        let layout = Layout::new(rect);
        let painter = ui.painter();
        let center_x = rect.center().x;
        let time = self.time;

        // === BACKGROUND ===
        painter.rect_filled(rect, 0.0, ARCADE_DARK);
        for i in 0..20 {
            let alpha = ((20 - i) as f32 / 20.0 * 30.0) as u8;
            let y = layout.y_from_bottom(i as f32 * 40.0);
            painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(rect.min.x, y - layout.px(40.0)),
                    Pos2::new(rect.max.x, y),
                ),
                0.0,
                Color32::from_rgba_premultiplied(80, 0, 120, alpha),
            );
        }

        // === PERSPECTIVE GRID ===
        let grid_top = layout.y(1200.0);
        let grid_bottom = rect.max.y;
        let vanishing_x = center_x;
        for i in 0..15 {
            let t = i as f32 / 14.0;
            let y = grid_top + (grid_bottom - grid_top) * t * t;
            let spread = layout.px(200.0 + (t * 800.0));
            painter.line_segment(
                [
                    Pos2::new(vanishing_x - spread, y),
                    Pos2::new(vanishing_x + spread, y),
                ],
                Stroke::new(1.0 + t, GRID_COLOR),
            );
        }
        for i in -8..=8 {
            let bottom_x = vanishing_x + (i as f32 * layout.px(120.0));
            painter.line_segment(
                [
                    Pos2::new(vanishing_x, grid_top),
                    Pos2::new(bottom_x, grid_bottom),
                ],
                Stroke::new(1.0, GRID_COLOR),
            );
        }

        // === STARFIELD ===
        for star in &self.stars {
            let twinkle = (time * 3.0 + star.x * 10.0).sin() * 0.3 + 0.7;
            let alpha = (f32::from(star.brightness) * twinkle) as u8;
            painter.circle_filled(
                Pos2::new(rect.min.x + star.x * rect.width(), rect.min.y + star.y * rect.height()),
                1.0 + (f32::from(star.brightness) / 255.0),
                Color32::from_rgba_premultiplied(alpha, alpha, alpha, alpha),
            );
        }

        // === DECORATIVE CORNER BRACKETS ===
        let bracket_size = layout.px(60.0);
        let bracket_thickness = layout.px(4.0);
        let margin = layout.px(30.0);

        let draw_bracket = |corner_x: f32, corner_y: f32, dx: f32, dy: f32, color: Color32| {
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(corner_x, corner_y),
                    Vec2::new(bracket_size * dx, bracket_thickness),
                ),
                0.0,
                color,
            );
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(corner_x, corner_y),
                    Vec2::new(bracket_thickness, bracket_size * dy),
                ),
                0.0,
                color,
            );
        };

        draw_bracket(rect.min.x + margin, rect.min.y + margin, 1.0, 1.0, NEON_CYAN);
        draw_bracket(rect.max.x - margin - bracket_size, rect.min.y + margin, 1.0, 1.0, NEON_CYAN);
        painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.max.x - margin - bracket_thickness, rect.min.y + margin),
                Vec2::new(bracket_thickness, bracket_size),
            ),
            0.0,
            NEON_CYAN,
        );
        draw_bracket(
            rect.min.x + margin,
            rect.max.y - margin - bracket_thickness,
            1.0,
            -1.0,
            NEON_MAGENTA,
        );
        painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.min.x + margin, rect.max.y - margin - bracket_size),
                Vec2::new(bracket_thickness, bracket_size),
            ),
            0.0,
            NEON_MAGENTA,
        );
        draw_bracket(
            rect.max.x - margin - bracket_size,
            rect.max.y - margin - bracket_thickness,
            1.0,
            -1.0,
            NEON_MAGENTA,
        );
        painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.max.x - margin - bracket_thickness, rect.max.y - margin - bracket_size),
                Vec2::new(bracket_thickness, bracket_size),
            ),
            0.0,
            NEON_MAGENTA,
        );

        // === MAIN TITLE ===
        let title_y = layout.y(180.0);
        self.draw_glitch_title(painter, center_x, title_y, &config.title, layout.px(90.0));
        self.draw_glitch_title(
            painter,
            center_x,
            title_y + layout.px(80.0),
            &config.subtitle,
            layout.px(48.0),
        );

        // Decorative line under title
        let line_y = title_y + layout.px(130.0);
        let line_width = layout.px(400.0);
        painter.rect_filled(
            Rect::from_center_size(Pos2::new(center_x, line_y), Vec2::new(line_width, layout.px(3.0))),
            0.0,
            NEON_CYAN,
        );
        painter.rect_filled(
            Rect::from_center_size(
                Pos2::new(center_x - line_width / 2.0, line_y),
                Vec2::new(layout.px(8.0), layout.px(12.0)),
            ),
            0.0,
            NEON_CYAN,
        );
        painter.rect_filled(
            Rect::from_center_size(
                Pos2::new(center_x + line_width / 2.0, line_y),
                Vec2::new(layout.px(8.0), layout.px(12.0)),
            ),
            0.0,
            NEON_CYAN,
        );

        // === SELECT GAME HEADER ===
        let header_y = layout.y(420.0);
        painter.text(
            Pos2::new(center_x, header_y),
            Align2::CENTER_CENTER,
            "< SELECT GAME >",
            FontId::monospace(layout.px(32.0)),
            NEON_YELLOW,
        );

        // === GAME LIST ===
        let list_start_y = layout.y(520.0);
        let item_height = layout.px(95.0);
        let item_width = layout.px(700.0);
        for (i, game) in games.iter().enumerate() {
            let y = list_start_y + (i as f32 * item_height);
            let is_selected = i == self.selected_index;
            let item_rect = Rect::from_center_size(
                Pos2::new(center_x, y + item_height / 2.0),
                Vec2::new(item_width, item_height - layout.px(10.0)),
            );

            if is_selected {
                let pulse = (time * 4.0).sin() * 0.3 + 0.7;
                let glow_alpha = (pulse * 60.0) as u8;
                let glow_rect = item_rect.expand(layout.px(8.0));
                painter.rect_filled(
                    glow_rect,
                    layout.px(4.0),
                    Color32::from_rgba_premultiplied(0, 255, 255, glow_alpha),
                );
                painter.rect_filled(
                    item_rect,
                    layout.px(4.0),
                    Color32::from_rgba_premultiplied(0, 40, 60, 200),
                );
                painter.rect_stroke(
                    item_rect,
                    layout.px(4.0),
                    Stroke::new(layout.px(3.0), NEON_CYAN),
                    StrokeKind::Inside,
                );
                let arrow_pulse = (time * 6.0).sin() * layout.px(8.0);
                painter.text(
                    Pos2::new(item_rect.min.x - layout.px(30.0) - arrow_pulse, item_rect.center().y),
                    Align2::CENTER_CENTER,
                    ">>",
                    FontId::monospace(layout.px(36.0)),
                    NEON_CYAN,
                );
                painter.text(
                    Pos2::new(item_rect.max.x + layout.px(30.0) + arrow_pulse, item_rect.center().y),
                    Align2::CENTER_CENTER,
                    "<<",
                    FontId::monospace(layout.px(36.0)),
                    NEON_CYAN,
                );
            } else {
                painter.rect_stroke(
                    item_rect,
                    layout.px(4.0),
                    Stroke::new(1.0, Color32::from_rgb(60, 60, 80)),
                    StrokeKind::Inside,
                );
            }

            let name_color = if is_selected {
                Color32::WHITE
            } else {
                Color32::from_rgb(160, 160, 160)
            };
            painter.text(
                Pos2::new(center_x, item_rect.center().y - layout.px(10.0)),
                Align2::CENTER_CENTER,
                game.name(),
                FontId::monospace(layout.px(36.0)),
                name_color,
            );

            let desc = game.description();
            let year = game.year();
            let desc_line = match (desc.is_empty(), year.is_empty()) {
                (true, true) => String::new(),
                (false, true) => desc.to_string(),
                (true, false) => format!("({year})"),
                (false, false) => format!("{desc} ({year})"),
            };
            if !desc_line.is_empty() {
                painter.text(
                    Pos2::new(center_x, item_rect.center().y + layout.px(16.0)),
                    Align2::CENTER_CENTER,
                    &desc_line,
                    FontId::monospace(layout.px(16.0)),
                    if is_selected {
                        Color32::from_rgb(150, 150, 150)
                    } else {
                        Color32::from_rgb(80, 80, 80)
                    },
                );
            }
        }

        // === BLINKING PRESS-TO-PLAY ===
        let blink = (time * 2.5).sin() > 0.0;
        if blink {
            painter.text(
                Pos2::new(center_x, layout.y(1050.0)),
                Align2::CENTER_CENTER,
                &config.prompt,
                FontId::monospace(layout.px(28.0)),
                NEON_YELLOW,
            );
        }

        // === CONTROLS INFO ===
        let controls_y = layout.y_from_bottom(180.0);
        painter.text(
            Pos2::new(center_x, controls_y),
            Align2::CENTER_CENTER,
            &config.controls_select,
            FontId::monospace(layout.px(22.0)),
            Color32::from_rgb(100, 100, 100),
        );
        painter.text(
            Pos2::new(center_x, controls_y + layout.px(35.0)),
            Align2::CENTER_CENTER,
            &config.controls_start,
            FontId::monospace(layout.px(22.0)),
            Color32::from_rgb(100, 100, 100),
        );

        painter.text(
            Pos2::new(center_x, layout.y_from_bottom(60.0)),
            Align2::CENTER_CENTER,
            &config.exit_hint,
            FontId::monospace(layout.px(18.0)),
            Color32::from_rgb(60, 60, 60),
        );

        // === CRT SCANLINE OVERLAY ===
        let mut sy = rect.min.y;
        while sy < rect.max.y {
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(rect.min.x, sy),
                    Vec2::new(rect.width(), 1.0),
                ),
                0.0,
                SCANLINE_COLOR,
            );
            sy += 3.0;
        }

        // === CRT HUM BARS ===
        for hum_bar in &self.hum_bars {
            let y_pos = rect.max.y - (hum_bar.phase * rect.height() * 2.0);
            if y_pos > rect.min.y - 20.0 && y_pos < rect.max.y + 20.0 {
                let bar_height = layout.px(4.0);
                let jitter = hum_bar.jitter_x;
                painter.rect_filled(
                    Rect::from_min_size(
                        Pos2::new(rect.min.x + jitter, y_pos - layout.px(3.0)),
                        Vec2::new(rect.width(), bar_height + layout.px(6.0)),
                    ),
                    0.0,
                    Color32::from_rgba_unmultiplied(0, 255, 0, 6),
                );
                painter.rect_filled(
                    Rect::from_min_size(
                        Pos2::new(rect.min.x + jitter, y_pos),
                        Vec2::new(rect.width(), bar_height),
                    ),
                    0.0,
                    Color32::from_rgba_unmultiplied(0, 255, 0, 18),
                );
            }
        }

        // === ROLLING INTERFERENCE ===
        for roll in &self.rolling_interference {
            let y_pos =
                rect.max.y - (roll.phase * rect.height() * 3.0) + rect.height();
            let band_height = layout.px(250.0);
            if y_pos > rect.min.y - band_height && y_pos < rect.max.y + band_height {
                painter.rect_filled(
                    Rect::from_min_size(
                        Pos2::new(rect.min.x, y_pos),
                        Vec2::new(rect.width(), band_height * 0.45),
                    ),
                    0.0,
                    Color32::from_rgba_unmultiplied(0, 255, 255, 4),
                );
                painter.rect_filled(
                    Rect::from_min_size(
                        Pos2::new(rect.min.x, y_pos + band_height * 0.45),
                        Vec2::new(rect.width(), band_height * 0.1),
                    ),
                    0.0,
                    Color32::from_rgba_unmultiplied(0, 255, 255, 10),
                );
                painter.rect_filled(
                    Rect::from_min_size(
                        Pos2::new(rect.min.x, y_pos + band_height * 0.55),
                        Vec2::new(rect.width(), band_height * 0.45),
                    ),
                    0.0,
                    Color32::from_rgba_unmultiplied(0, 255, 255, 4),
                );
            }
        }

        // === CRT VIGNETTE ===
        for i in 0..30 {
            let alpha = ((30 - i) as f32 / 30.0 * 80.0) as u8;
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(rect.min.x, rect.min.y + (i as f32 * 3.0)),
                    Vec2::new(rect.width(), 3.0),
                ),
                0.0,
                Color32::from_rgba_premultiplied(0, 0, 0, alpha),
            );
        }
        for i in 0..30 {
            let alpha = ((30 - i) as f32 / 30.0 * 80.0) as u8;
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(rect.min.x, rect.max.y - ((i + 1) as f32 * 3.0)),
                    Vec2::new(rect.width(), 3.0),
                ),
                0.0,
                Color32::from_rgba_premultiplied(0, 0, 0, alpha),
            );
        }
    }
}

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}
