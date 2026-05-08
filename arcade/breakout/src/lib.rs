//! Breakout cartridge.
//!
//! Classic brick-breaking game rendered with `egui::Painter`. Designed for
//! the 1080×1920 native rect; render scales linearly to other window sizes.

use arcade_cart::{Game, Input};
use egui::{Color32, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};

const PADDLE_WIDTH: f32 = 160.0;
const PADDLE_HEIGHT: f32 = 28.0;
const BALL_RADIUS: f32 = 14.0;
const BALL_SPEED_INITIAL: f32 = 500.0;
const BALL_SPEED_MAX: f32 = 900.0;
const BALL_SPEED_INCREMENT: f32 = 15.0;
const BRICK_ROWS: usize = 8;
const BRICK_COLS: usize = 7;
const BRICK_HEIGHT: f32 = 45.0;
const BRICK_PADDING: f32 = 6.0;
const BRICK_TOP_MARGIN: f32 = 200.0;

/// Pixels of paddle travel per radian of rotation input.
/// Tuned so 120° (~2.094 rad) sweeps the full 1080-pixel width.
const PADDLE_PIXELS_PER_RADIAN: f32 = 515.7;

/// Smoothing factor: higher = snappier paddle, lower = smoother.
const PADDLE_SMOOTHING: f32 = 15.0;

pub struct Breakout {
    paddle_x: f32,
    paddle_target_x: f32,
    ball_pos: Vec2,
    ball_vel: Vec2,
    ball_speed: f32,
    bricks: Vec<Brick>,
    score: u32,
    game_over: bool,
    won: bool,
    screen_width: f32,
    screen_height: f32,
}

struct Brick {
    x: f32,
    y: f32,
    width: f32,
    alive: bool,
    color: Color32,
}

impl Breakout {
    #[must_use]
    pub fn new() -> Self {
        let center_x = arcade_cart::DESIGN_WIDTH * 0.5;
        let vel_component = BALL_SPEED_INITIAL / std::f32::consts::SQRT_2;
        Self {
            paddle_x: center_x,
            paddle_target_x: center_x,
            ball_pos: Vec2::new(center_x, arcade_cart::DESIGN_HEIGHT * 0.625),
            ball_vel: Vec2::new(vel_component, -vel_component),
            ball_speed: BALL_SPEED_INITIAL,
            bricks: Vec::new(),
            score: 0,
            game_over: false,
            won: false,
            screen_width: arcade_cart::DESIGN_WIDTH,
            screen_height: arcade_cart::DESIGN_HEIGHT,
        }
    }

    fn init_bricks(&mut self) {
        self.bricks.clear();
        let brick_width = (self.screen_width - (BRICK_COLS + 1) as f32 * BRICK_PADDING)
            / BRICK_COLS as f32;

        // Wong/Tol-style colorblind-safe palette.
        let colors = [
            Color32::from_rgb(213, 94, 0),
            Color32::from_rgb(230, 159, 0),
            Color32::from_rgb(204, 187, 68),
            Color32::from_rgb(0, 158, 115),
            Color32::from_rgb(86, 180, 233),
            Color32::from_rgb(0, 114, 178),
            Color32::from_rgb(204, 121, 167),
            Color32::from_rgb(170, 51, 119),
        ];

        for row in 0..BRICK_ROWS {
            for col in 0..BRICK_COLS {
                let x = BRICK_PADDING + col as f32 * (brick_width + BRICK_PADDING);
                let y = BRICK_TOP_MARGIN + row as f32 * (BRICK_HEIGHT + BRICK_PADDING);

                self.bricks.push(Brick {
                    x,
                    y,
                    width: brick_width,
                    alive: true,
                    color: colors[row % colors.len()],
                });
            }
        }
    }

    fn update_physics(&mut self, dt: f32) {
        if self.game_over || self.won {
            return;
        }

        self.ball_pos += self.ball_vel * dt;

        if self.ball_pos.x - BALL_RADIUS < 0.0 || self.ball_pos.x + BALL_RADIUS > self.screen_width {
            self.ball_vel.x = -self.ball_vel.x;
            self.ball_pos.x = self.ball_pos.x.clamp(BALL_RADIUS, self.screen_width - BALL_RADIUS);
        }

        if self.ball_pos.y - BALL_RADIUS < 0.0 {
            self.ball_vel.y = -self.ball_vel.y;
            self.ball_pos.y = BALL_RADIUS;
        }

        if self.ball_pos.y > self.screen_height {
            self.game_over = true;
            return;
        }

        let paddle_y = self.screen_height - 150.0;
        if self.ball_pos.y + BALL_RADIUS >= paddle_y
            && self.ball_pos.y - BALL_RADIUS <= paddle_y + PADDLE_HEIGHT
            && self.ball_pos.x >= self.paddle_x - PADDLE_WIDTH / 2.0
            && self.ball_pos.x <= self.paddle_x + PADDLE_WIDTH / 2.0
        {
            let hit_pos = (self.ball_pos.x - self.paddle_x) / (PADDLE_WIDTH / 2.0);
            let angle = hit_pos;
            self.ball_vel.x = self.ball_speed * angle.sin();
            self.ball_vel.y = -self.ball_speed * angle.cos().abs();
            self.ball_pos.y = paddle_y - BALL_RADIUS;
        }

        for brick in &mut self.bricks {
            if !brick.alive {
                continue;
            }

            let brick_rect = Rect::from_min_size(
                Pos2::new(brick.x, brick.y),
                Vec2::new(brick.width, BRICK_HEIGHT),
            );
            let ball_rect = Rect::from_center_size(
                Pos2::new(self.ball_pos.x, self.ball_pos.y),
                Vec2::splat(BALL_RADIUS * 2.0),
            );

            if brick_rect.intersects(ball_rect) {
                brick.alive = false;
                self.score += 10;
                self.ball_speed = (self.ball_speed + BALL_SPEED_INCREMENT).min(BALL_SPEED_MAX);

                let current_speed = self.ball_vel.length();
                if current_speed > 0.0 {
                    let scale = self.ball_speed / current_speed;
                    self.ball_vel.x *= scale;
                    self.ball_vel.y *= scale;
                }

                self.ball_vel.y = -self.ball_vel.y;
                break;
            }
        }

        if self.bricks.iter().all(|b| !b.alive) {
            self.won = true;
        }
    }
}

impl Default for Breakout {
    fn default() -> Self {
        Self::new()
    }
}

impl Game for Breakout {
    fn update(&mut self, dt: f32, input: &Input) -> bool {
        if self.bricks.is_empty() {
            self.init_bricks();
        }

        self.paddle_target_x += input.rotation * PADDLE_PIXELS_PER_RADIAN;
        self.paddle_target_x = self.paddle_target_x.clamp(
            PADDLE_WIDTH / 2.0,
            self.screen_width - PADDLE_WIDTH / 2.0,
        );

        let smoothing = (PADDLE_SMOOTHING * dt).min(1.0);
        self.paddle_x += (self.paddle_target_x - self.paddle_x) * smoothing;

        if input.action_pressed && (self.game_over || self.won) {
            *self = Self::new();
            return false;
        }

        self.update_physics(dt);

        input.exit_requested || input.menu_requested
    }

    fn render(&self, ui: &mut Ui) {
        let rect = ui.available_rect_before_wrap();
        let painter = ui.painter();

        let sx = rect.width() / self.screen_width;
        let sy = rect.height() / self.screen_height;
        let uniform = sx.min(sy);

        painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_rgb(10, 10, 20));

        // Paddle
        let paddle_y_game = self.screen_height - 150.0;
        let paddle_center = Pos2::new(
            rect.min.x + self.paddle_x * sx,
            rect.min.y + paddle_y_game * sy,
        );
        let paddle_rect = Rect::from_center_size(
            paddle_center,
            Vec2::new(PADDLE_WIDTH * uniform, PADDLE_HEIGHT * uniform),
        );
        let glow_rect = paddle_rect.expand(4.0 * uniform);
        painter.rect_filled(
            glow_rect,
            CornerRadius::same(4),
            Color32::from_rgba_premultiplied(100, 100, 255, 60),
        );
        painter.rect_filled(
            paddle_rect,
            CornerRadius::same(4),
            Color32::from_rgb(220, 220, 240),
        );

        // Ball
        painter.circle_filled(
            Pos2::new(
                rect.min.x + self.ball_pos.x * sx,
                rect.min.y + self.ball_pos.y * sy,
            ),
            BALL_RADIUS * uniform,
            Color32::from_rgb(230, 159, 0),
        );

        // Bricks
        for brick in &self.bricks {
            if !brick.alive {
                continue;
            }
            let brick_rect = Rect::from_min_size(
                Pos2::new(rect.min.x + brick.x * sx, rect.min.y + brick.y * sy),
                Vec2::new(brick.width * sx, BRICK_HEIGHT * sy),
            );
            painter.rect_filled(brick_rect, CornerRadius::ZERO, brick.color);
            painter.rect_stroke(
                brick_rect,
                CornerRadius::ZERO,
                Stroke::new(1.0, Color32::from_rgb(50, 50, 60)),
                StrokeKind::Inside,
            );
        }

        // Score
        painter.text(
            Pos2::new(rect.min.x + 20.0, rect.min.y + 40.0 * uniform),
            egui::Align2::LEFT_TOP,
            format!("SCORE: {}", self.score),
            egui::FontId::proportional(32.0 * uniform),
            Color32::WHITE,
        );

        // End-of-game messages
        if self.game_over {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "GAME OVER\n\nPress button to restart",
                egui::FontId::proportional(48.0 * uniform),
                Color32::from_rgb(213, 94, 0),
            );
        } else if self.won {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "YOU WIN!\n\nPress button to restart",
                egui::FontId::proportional(48.0 * uniform),
                Color32::from_rgb(0, 158, 115),
            );
        }
    }

    fn name(&self) -> &str {
        "BREAKOUT"
    }

    fn description(&self) -> &str {
        "SMASH ALL THE BRICKS"
    }

    fn year(&self) -> &str {
        "1976"
    }

    fn debug_stats(&self) -> Option<String> {
        let alive = self.bricks.iter().filter(|b| b.alive).count();
        Some(format!(
            "bricks={}/{}, score={}",
            alive,
            self.bricks.len(),
            self.score
        ))
    }
}
