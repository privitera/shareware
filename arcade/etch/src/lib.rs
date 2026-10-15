//! Sketch-pad cartridge.
//!
//! Two-axis drawing toy: left input controls the X axis of the cursor,
//! right input controls the Y axis. Press the action button to "shake" and
//! clear the canvas.
//!
//! Designed for the 1080×1920 native rect; render scales linearly.

use arcade_cart::{Game, Input};
use egui::{Color32, Pos2, Rect, Stroke, Ui, Vec2};

const FRAME_BORDER: f32 = 40.0;
const DRAW_MARGIN_TOP: f32 = 200.0;
const DRAW_MARGIN_BOTTOM: f32 = 350.0;
const DRAW_MARGIN_SIDE: f32 = 60.0;
const CURSOR_SIZE: f32 = 4.0;
const LINE_THICKNESS: f32 = 3.0;
const MAX_SEGMENTS: usize = 10000;
const CURSOR_PIXELS_PER_RADIAN: f32 = 200.0;
const SHAKE_DURATION: f32 = 1.5;
const SHAKE_INTENSITY: f32 = 20.0;

const FRAME_RED: Color32 = Color32::from_rgb(200, 30, 30);
const SCREEN_GRAY: Color32 = Color32::from_rgb(180, 180, 170);
const LINE_DARK: Color32 = Color32::from_rgb(60, 60, 60);
const DIAL_LIGHT: Color32 = Color32::from_rgb(240, 240, 240);
const DIAL_SHADOW: Color32 = Color32::from_rgb(180, 180, 180);

#[derive(Clone, Copy)]
struct Segment {
    start: Pos2,
    end: Pos2,
}

/// Sketch-pad cartridge state.
pub struct Sketch {
    cursor_x: f32,
    cursor_y: f32,
    segments: Vec<Segment>,
    last_pos: Option<Pos2>,
    shaking: bool,
    shake_timer: f32,
    screen_width: f32,
    screen_height: f32,
    draw_left: f32,
    draw_right: f32,
    draw_top: f32,
    draw_bottom: f32,
}

impl Sketch {
    #[must_use]
    pub fn new() -> Self {
        let screen_width = arcade_cart::DESIGN_WIDTH;
        let screen_height = arcade_cart::DESIGN_HEIGHT;

        let draw_left = FRAME_BORDER + DRAW_MARGIN_SIDE;
        let draw_right = screen_width - FRAME_BORDER - DRAW_MARGIN_SIDE;
        let draw_top = FRAME_BORDER + DRAW_MARGIN_TOP;
        let draw_bottom = screen_height - FRAME_BORDER - DRAW_MARGIN_BOTTOM;

        Self {
            cursor_x: f32::midpoint(draw_left, draw_right),
            cursor_y: f32::midpoint(draw_top, draw_bottom),
            segments: Vec::with_capacity(MAX_SEGMENTS),
            last_pos: None,
            shaking: false,
            shake_timer: 0.0,
            screen_width,
            screen_height,
            draw_left,
            draw_right,
            draw_top,
            draw_bottom,
        }
    }

    fn clear(&mut self) {
        self.segments.clear();
        self.last_pos = None;
        self.cursor_x = f32::midpoint(self.draw_left, self.draw_right);
        self.cursor_y = f32::midpoint(self.draw_top, self.draw_bottom);
    }

    fn add_segment(&mut self, start: Pos2, end: Pos2) {
        let dx = (end.x - start.x).abs();
        let dy = (end.y - start.y).abs();
        if dx > 0.5 || dy > 0.5 {
            if self.segments.len() >= MAX_SEGMENTS {
                let remove_count = MAX_SEGMENTS / 10;
                self.segments.drain(0..remove_count);
            }
            self.segments.push(Segment { start, end });
        }
    }
}

impl Default for Sketch {
    fn default() -> Self {
        Self::new()
    }
}

impl Game for Sketch {
    fn update(&mut self, dt: f32, input: &Input) -> bool {
        if input.exit_requested || input.menu_requested {
            return true;
        }

        if self.shaking {
            self.shake_timer -= dt;
            if self.shake_timer <= 0.0 {
                self.shaking = false;
                self.clear();
            }
            return false;
        }

        if input.action_pressed {
            self.shaking = true;
            self.shake_timer = SHAKE_DURATION;
            return false;
        }

        // Left input drives X, right input drives Y (Y inverted for natural feel).
        self.cursor_x += input.rotation_left * CURSOR_PIXELS_PER_RADIAN;
        self.cursor_y -= input.rotation_right * CURSOR_PIXELS_PER_RADIAN;

        self.cursor_x = self.cursor_x.clamp(self.draw_left, self.draw_right);
        self.cursor_y = self.cursor_y.clamp(self.draw_top, self.draw_bottom);

        let current_pos = Pos2::new(self.cursor_x, self.cursor_y);
        if let Some(last) = self.last_pos {
            self.add_segment(last, current_pos);
        }
        self.last_pos = Some(current_pos);

        false
    }

    fn render(&self, ui: &mut Ui) {
        let rect = ui.available_rect_before_wrap();
        let painter = ui.painter();

        let sx = rect.width() / self.screen_width;
        let sy = rect.height() / self.screen_height;
        let uniform = sx.min(sy);

        let to_screen = |gx: f32, gy: f32| -> Pos2 {
            Pos2::new(rect.min.x + gx * sx, rect.min.y + gy * sy)
        };

        // Shake offset, in screen pixels.
        let shake_offset = if self.shaking {
            let progress = self.shake_timer / SHAKE_DURATION;
            let shake_freq = 30.0;
            let intensity = SHAKE_INTENSITY * progress * uniform;
            Vec2::new(
                (self.shake_timer * shake_freq).sin() * intensity,
                (self.shake_timer * shake_freq * 1.3).cos() * intensity * 0.7,
            )
        } else {
            Vec2::ZERO
        };

        painter.rect_filled(rect, 0.0, Color32::from_rgb(40, 40, 40));

        // Frame
        let frame_min = to_screen(FRAME_BORDER, FRAME_BORDER) + shake_offset;
        let frame_max = to_screen(self.screen_width - FRAME_BORDER, self.screen_height - FRAME_BORDER)
            + shake_offset;
        let frame_rect = Rect::from_min_max(frame_min, frame_max);
        painter.rect_filled(frame_rect, 20.0 * uniform, FRAME_RED);

        // Drawing surface
        let screen_rect = Rect::from_min_max(
            to_screen(self.draw_left - 10.0, self.draw_top - 10.0) + shake_offset,
            to_screen(self.draw_right + 10.0, self.draw_bottom + 10.0) + shake_offset,
        );
        painter.rect_filled(screen_rect, 8.0 * uniform, SCREEN_GRAY);

        // Lines
        if !self.shaking || self.shake_timer > SHAKE_DURATION * 0.3 {
            let alpha = if self.shaking {
                ((self.shake_timer / SHAKE_DURATION - 0.3) / 0.7 * 255.0) as u8
            } else {
                255
            };
            let line_color = Color32::from_rgba_unmultiplied(
                LINE_DARK.r(),
                LINE_DARK.g(),
                LINE_DARK.b(),
                alpha,
            );

            for seg in &self.segments {
                painter.line_segment(
                    [
                        to_screen(seg.start.x, seg.start.y) + shake_offset,
                        to_screen(seg.end.x, seg.end.y) + shake_offset,
                    ],
                    Stroke::new(LINE_THICKNESS * uniform, line_color),
                );
            }
        }

        // Cursor
        if !self.shaking {
            let cursor_pos = to_screen(self.cursor_x, self.cursor_y) + shake_offset;
            painter.circle_filled(cursor_pos, CURSOR_SIZE * uniform, LINE_DARK);
            painter.circle_stroke(
                cursor_pos,
                (CURSOR_SIZE + 2.0) * uniform,
                Stroke::new(1.5 * uniform, Color32::from_rgb(100, 100, 100)),
            );
        }

        // L/R axis indicators (decorative dials at the bottom)
        let dial_y_game = self.screen_height - FRAME_BORDER - 150.0;
        let dial_radius_game = 60.0;
        let left_dial_x = FRAME_BORDER + 150.0;
        let right_dial_x = self.screen_width - FRAME_BORDER - 150.0;
        let dial_radius = dial_radius_game * uniform;

        let draw_dial = |painter: &egui::Painter, gx: f32, gy: f32| {
            let center = to_screen(gx, gy) + shake_offset;
            painter.circle_filled(center, dial_radius, DIAL_LIGHT);
            painter.circle_stroke(center, dial_radius, Stroke::new(3.0 * uniform, DIAL_SHADOW));
            for i in 0..8 {
                let angle = i as f32 * std::f32::consts::PI / 4.0;
                let inner = dial_radius * 0.5;
                let outer = dial_radius * 0.8;
                painter.line_segment(
                    [
                        Pos2::new(center.x + angle.cos() * inner, center.y + angle.sin() * inner),
                        Pos2::new(center.x + angle.cos() * outer, center.y + angle.sin() * outer),
                    ],
                    Stroke::new(2.0 * uniform, DIAL_SHADOW),
                );
            }
        };

        draw_dial(painter, left_dial_x, dial_y_game);
        draw_dial(painter, right_dial_x, dial_y_game);

        // Title
        painter.text(
            to_screen(self.screen_width * 0.5, FRAME_BORDER + 100.0) + shake_offset,
            egui::Align2::CENTER_CENTER,
            "SKETCH",
            egui::FontId::proportional(56.0 * uniform),
            Color32::from_rgb(255, 220, 100),
        );

        // Dial labels
        painter.text(
            to_screen(left_dial_x, dial_y_game + dial_radius_game + 30.0) + shake_offset,
            egui::Align2::CENTER_CENTER,
            "L  ← →",
            egui::FontId::proportional(28.0 * uniform),
            DIAL_SHADOW,
        );
        painter.text(
            to_screen(right_dial_x, dial_y_game + dial_radius_game + 30.0) + shake_offset,
            egui::Align2::CENTER_CENTER,
            "R  ↑ ↓",
            egui::FontId::proportional(28.0 * uniform),
            DIAL_SHADOW,
        );

        // Footer / shake indicator
        if self.shaking {
            painter.text(
                to_screen(self.screen_width * 0.5, self.screen_height * 0.5) + shake_offset,
                egui::Align2::CENTER_CENTER,
                "SHAKE!",
                egui::FontId::proportional(72.0 * uniform),
                Color32::from_rgb(255, 255, 200),
            );
        } else {
            painter.text(
                to_screen(self.screen_width * 0.5, self.screen_height - 50.0) + shake_offset,
                egui::Align2::CENTER_CENTER,
                "PRESS TO SHAKE & CLEAR",
                egui::FontId::proportional(22.0 * uniform),
                Color32::GRAY,
            );
        }

        // Segment count
        painter.text(
            to_screen(self.draw_right - 10.0, self.draw_bottom + 30.0) + shake_offset,
            egui::Align2::RIGHT_TOP,
            format!("{} lines", self.segments.len()),
            egui::FontId::proportional(16.0 * uniform),
            Color32::from_rgba_unmultiplied(100, 100, 100, 150),
        );
    }

    fn name(&self) -> &str {
        "SKETCH"
    }

    fn description(&self) -> &str {
        "L/R TO DRAW"
    }

    fn year(&self) -> &str {
        "1960"
    }

    fn debug_stats(&self) -> Option<String> {
        Some(format!(
            "segments={}/{}, cursor=({:.0},{:.0}), shaking={}",
            self.segments.len(),
            MAX_SEGMENTS,
            self.cursor_x,
            self.cursor_y,
            self.shaking
        ))
    }
}
