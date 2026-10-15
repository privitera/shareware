//! Pong cartridge.
//!
//! Faithful recreation of the 1972 Atari Pong arcade game — white rectangles
//! on a black background. Two modes: 1P vs computer, or 2P (left input vs
//! right input).
//!
//! Designed for the 1080×1920 native rect; render scales linearly.

use arcade_cart::{Game, Input};
use egui::{Color32, Pos2, Rect, StrokeKind, Ui, Vec2};

const PADDLE_WIDTH: f32 = 24.0;
const PADDLE_HEIGHT: f32 = 120.0;
const PADDLE_MARGIN: f32 = 40.0;
const BALL_SIZE: f32 = 24.0;
const PADDLE_SPEED: f32 = 600.0;
const BALL_SPEED_INITIAL: f32 = 400.0;
const BALL_SPEED_MAX: f32 = 800.0;
const BALL_ACCELERATION: f32 = 20.0;
const AI_REACTION_DELAY: f32 = 0.1;
const AI_ERROR_MARGIN: f32 = 40.0;
const WALL_THICKNESS: f32 = 16.0;
const CENTER_LINE_SEGMENT: f32 = 30.0;
const CENTER_LINE_GAP: f32 = 20.0;
const WINNING_SCORE: u32 = 11;
const PADDLE_PIXELS_PER_RADIAN: f32 = 400.0;
const PADDLE_SMOOTHING: f32 = 12.0;
const MENU_ROTATION_THRESHOLD: f32 = 0.5;
const BUTTON_DEBOUNCE_COOLDOWN: f32 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PongMode {
    OnePlayer,
    TwoPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameState {
    ModeSelect,
    Playing,
    GameOver,
}

pub struct Pong {
    state: GameState,
    mode: PongMode,
    menu_selection: usize,
    menu_rotation_accumulator: f32,
    button_debounce_timer: f32,

    left_paddle_y: f32,
    left_paddle_target_y: f32,
    right_paddle_y: f32,
    right_paddle_target_y: f32,

    ball_x: f32,
    ball_y: f32,
    ball_vx: f32,
    ball_vy: f32,
    ball_speed: f32,

    left_score: u32,
    right_score: u32,

    serving: bool,
    serve_timer: f32,
    left_won: bool,

    ai_target_y: f32,
    ai_reaction_timer: f32,

    screen_width: f32,
    screen_height: f32,
    play_top: f32,
    play_bottom: f32,
}

impl Pong {
    #[must_use]
    pub fn new() -> Self {
        let screen_width = arcade_cart::DESIGN_WIDTH;
        let screen_height = arcade_cart::DESIGN_HEIGHT;
        let center_y = screen_height * 0.5;

        Self {
            state: GameState::ModeSelect,
            mode: PongMode::OnePlayer,
            menu_selection: 0,
            menu_rotation_accumulator: 0.0,
            button_debounce_timer: BUTTON_DEBOUNCE_COOLDOWN,

            left_paddle_y: center_y,
            left_paddle_target_y: center_y,
            right_paddle_y: center_y,
            right_paddle_target_y: center_y,
            ball_x: screen_width * 0.5,
            ball_y: center_y,
            ball_vx: 0.0,
            ball_vy: 0.0,
            ball_speed: BALL_SPEED_INITIAL,
            left_score: 0,
            right_score: 0,
            serving: true,
            serve_timer: 1.0,
            left_won: false,
            ai_target_y: center_y,
            ai_reaction_timer: 0.0,
            screen_width,
            screen_height,
            play_top: WALL_THICKNESS,
            play_bottom: screen_height - WALL_THICKNESS,
        }
    }

    fn start_game(&mut self) {
        let center_y = self.screen_height * 0.5;
        self.state = GameState::Playing;
        self.left_paddle_y = center_y;
        self.left_paddle_target_y = center_y;
        self.right_paddle_y = center_y;
        self.right_paddle_target_y = center_y;
        self.ball_x = self.screen_width * 0.5;
        self.ball_y = center_y;
        self.ball_vx = 0.0;
        self.ball_vy = 0.0;
        self.ball_speed = BALL_SPEED_INITIAL;
        self.left_score = 0;
        self.right_score = 0;
        self.serving = true;
        self.serve_timer = 1.0;
        self.ai_target_y = center_y;
        self.ai_reaction_timer = 0.0;
    }

    fn serve(&mut self, to_left: bool) {
        self.ball_x = self.screen_width * 0.5;
        self.ball_y = self.screen_height * 0.5;
        self.ball_speed = BALL_SPEED_INITIAL;
        let angle = if to_left { 0.8 } else { -0.8 };
        let dir = if to_left { -1.0 } else { 1.0 };
        self.ball_vx = dir * self.ball_speed * 0.9;
        self.ball_vy = angle * self.ball_speed * 0.4;
        self.serving = false;
    }

    fn reset_for_point(&mut self) {
        self.serving = true;
        self.serve_timer = 1.0;
        self.ball_vx = 0.0;
        self.ball_vy = 0.0;
        self.ball_x = self.screen_width * 0.5;
        self.ball_y = self.screen_height * 0.5;
    }

    fn update_ai(&mut self, dt: f32) {
        if self.ball_vx > 0.0 {
            self.ai_reaction_timer -= dt;

            if self.ai_reaction_timer <= 0.0 {
                let time_to_reach = (self.screen_width - PADDLE_MARGIN - self.ball_x) / self.ball_vx;
                let predicted_y = self.ball_y + self.ball_vy * time_to_reach;
                let error = (self.ai_reaction_timer * 100.0).sin() * AI_ERROR_MARGIN;
                self.ai_target_y = predicted_y + error;
                self.ai_target_y = self.ai_target_y.clamp(
                    self.play_top + PADDLE_HEIGHT * 0.5,
                    self.play_bottom - PADDLE_HEIGHT * 0.5,
                );
                self.ai_reaction_timer = AI_REACTION_DELAY;
            }
        } else {
            self.ai_target_y = self.screen_height * 0.5;
        }

        let diff = self.ai_target_y - self.right_paddle_y;
        let max_move = PADDLE_SPEED * 0.8 * dt;
        self.right_paddle_y += diff.clamp(-max_move, max_move);
        self.right_paddle_y = self.right_paddle_y.clamp(
            self.play_top + PADDLE_HEIGHT * 0.5,
            self.play_bottom - PADDLE_HEIGHT * 0.5,
        );
    }

    fn update_ball(&mut self, dt: f32) {
        self.ball_x += self.ball_vx * dt;
        self.ball_y += self.ball_vy * dt;

        if self.ball_y - BALL_SIZE * 0.5 < self.play_top {
            self.ball_y = self.play_top + BALL_SIZE * 0.5;
            self.ball_vy = -self.ball_vy;
        }
        if self.ball_y + BALL_SIZE * 0.5 > self.play_bottom {
            self.ball_y = self.play_bottom - BALL_SIZE * 0.5;
            self.ball_vy = -self.ball_vy;
        }

        // Left paddle collision
        let left_paddle_x = PADDLE_MARGIN;
        if self.ball_x - BALL_SIZE * 0.5 < left_paddle_x + PADDLE_WIDTH
            && self.ball_x + BALL_SIZE * 0.5 > left_paddle_x
            && self.ball_y > self.left_paddle_y - PADDLE_HEIGHT * 0.5
            && self.ball_y < self.left_paddle_y + PADDLE_HEIGHT * 0.5
            && self.ball_vx < 0.0
        {
            self.ball_vx = -self.ball_vx;
            self.ball_x = left_paddle_x + PADDLE_WIDTH + BALL_SIZE * 0.5;
            let hit_pos = (self.ball_y - self.left_paddle_y) / (PADDLE_HEIGHT * 0.5);
            self.ball_vy = hit_pos * self.ball_speed * 0.6;
            self.ball_speed = (self.ball_speed + BALL_ACCELERATION).min(BALL_SPEED_MAX);
            let speed = (self.ball_vx.powi(2) + self.ball_vy.powi(2)).sqrt();
            let scale = self.ball_speed / speed;
            self.ball_vx *= scale;
            self.ball_vy *= scale;
        }

        // Right paddle collision
        let right_paddle_x = self.screen_width - PADDLE_MARGIN - PADDLE_WIDTH;
        if self.ball_x + BALL_SIZE * 0.5 > right_paddle_x
            && self.ball_x - BALL_SIZE * 0.5 < right_paddle_x + PADDLE_WIDTH
            && self.ball_y > self.right_paddle_y - PADDLE_HEIGHT * 0.5
            && self.ball_y < self.right_paddle_y + PADDLE_HEIGHT * 0.5
            && self.ball_vx > 0.0
        {
            self.ball_vx = -self.ball_vx;
            self.ball_x = right_paddle_x - BALL_SIZE * 0.5;
            let hit_pos = (self.ball_y - self.right_paddle_y) / (PADDLE_HEIGHT * 0.5);
            self.ball_vy = hit_pos * self.ball_speed * 0.6;
            self.ball_speed = (self.ball_speed + BALL_ACCELERATION).min(BALL_SPEED_MAX);
            let speed = (self.ball_vx.powi(2) + self.ball_vy.powi(2)).sqrt();
            let scale = self.ball_speed / speed;
            self.ball_vx *= scale;
            self.ball_vy *= scale;
        }

        if self.ball_x < 0.0 {
            self.right_score += 1;
            if self.right_score >= WINNING_SCORE {
                self.state = GameState::GameOver;
                self.left_won = false;
            } else {
                self.reset_for_point();
            }
        } else if self.ball_x > self.screen_width {
            self.left_score += 1;
            if self.left_score >= WINNING_SCORE {
                self.state = GameState::GameOver;
                self.left_won = true;
            } else {
                self.reset_for_point();
            }
        }
    }

    fn update_menu(&mut self, input: &Input) -> bool {
        self.menu_rotation_accumulator += input.rotation;

        if self.menu_rotation_accumulator > MENU_ROTATION_THRESHOLD {
            self.menu_rotation_accumulator = 0.0;
            self.menu_selection = (self.menu_selection + 1) % 2;
        } else if self.menu_rotation_accumulator < -MENU_ROTATION_THRESHOLD {
            self.menu_rotation_accumulator = 0.0;
            self.menu_selection = usize::from(self.menu_selection == 0);
        }

        if input.action_pressed {
            self.mode = if self.menu_selection == 0 {
                PongMode::OnePlayer
            } else {
                PongMode::TwoPlayer
            };
            self.start_game();
        }

        false
    }

    fn update_playing(&mut self, dt: f32, input: &Input) -> bool {
        if self.serving {
            self.serve_timer -= dt;
            if self.serve_timer <= 0.0 {
                self.serve((self.left_score + self.right_score).is_multiple_of(2));
            }
            return false;
        }

        match self.mode {
            PongMode::OnePlayer => {
                self.left_paddle_target_y -= input.rotation * PADDLE_PIXELS_PER_RADIAN;
                self.left_paddle_target_y = self.left_paddle_target_y.clamp(
                    self.play_top + PADDLE_HEIGHT * 0.5,
                    self.play_bottom - PADDLE_HEIGHT * 0.5,
                );
                let smoothing = (PADDLE_SMOOTHING * dt).min(1.0);
                self.left_paddle_y += (self.left_paddle_target_y - self.left_paddle_y) * smoothing;
                self.update_ai(dt);
            }
            PongMode::TwoPlayer => {
                self.left_paddle_target_y -= input.rotation_left * PADDLE_PIXELS_PER_RADIAN;
                self.left_paddle_target_y = self.left_paddle_target_y.clamp(
                    self.play_top + PADDLE_HEIGHT * 0.5,
                    self.play_bottom - PADDLE_HEIGHT * 0.5,
                );
                self.right_paddle_target_y -= input.rotation_right * PADDLE_PIXELS_PER_RADIAN;
                self.right_paddle_target_y = self.right_paddle_target_y.clamp(
                    self.play_top + PADDLE_HEIGHT * 0.5,
                    self.play_bottom - PADDLE_HEIGHT * 0.5,
                );
                let smoothing = (PADDLE_SMOOTHING * dt).min(1.0);
                self.left_paddle_y += (self.left_paddle_target_y - self.left_paddle_y) * smoothing;
                self.right_paddle_y += (self.right_paddle_target_y - self.right_paddle_y) * smoothing;
            }
        }

        self.update_ball(dt);

        false
    }

    fn render_menu(&self, ui: &mut Ui) {
        let rect = ui.available_rect_before_wrap();
        let painter = ui.painter();
        let sx = rect.width() / self.screen_width;
        let sy = rect.height() / self.screen_height;
        let uniform = sx.min(sy);
        let to_screen = |gx: f32, gy: f32| Pos2::new(rect.min.x + gx * sx, rect.min.y + gy * sy);

        painter.rect_filled(rect, 0.0, Color32::BLACK);

        let white = Color32::WHITE;
        let selected_color = Color32::from_rgb(0, 158, 115);

        painter.text(
            to_screen(self.screen_width * 0.5, 300.0),
            egui::Align2::CENTER_CENTER,
            "PONG",
            egui::FontId::monospace(96.0 * uniform),
            white,
        );

        let option_y_1p = 600.0;
        let option_y_2p = 750.0;

        let color_1p = if self.menu_selection == 0 { selected_color } else { white };
        let prefix_1p = if self.menu_selection == 0 { "▶ " } else { "  " };
        painter.text(
            to_screen(self.screen_width * 0.5, option_y_1p),
            egui::Align2::CENTER_CENTER,
            format!("{prefix_1p}1 PLAYER"),
            egui::FontId::monospace(48.0 * uniform),
            color_1p,
        );
        painter.text(
            to_screen(self.screen_width * 0.5, option_y_1p + 50.0),
            egui::Align2::CENTER_CENTER,
            "vs Computer",
            egui::FontId::proportional(28.0 * uniform),
            Color32::GRAY,
        );

        let color_2p = if self.menu_selection == 1 { selected_color } else { white };
        let prefix_2p = if self.menu_selection == 1 { "▶ " } else { "  " };
        painter.text(
            to_screen(self.screen_width * 0.5, option_y_2p),
            egui::Align2::CENTER_CENTER,
            format!("{prefix_2p}2 PLAYERS"),
            egui::FontId::monospace(48.0 * uniform),
            color_2p,
        );
        painter.text(
            to_screen(self.screen_width * 0.5, option_y_2p + 50.0),
            egui::Align2::CENTER_CENTER,
            "Left vs Right",
            egui::FontId::proportional(28.0 * uniform),
            Color32::GRAY,
        );

        let selection_y = if self.menu_selection == 0 { option_y_1p } else { option_y_2p };
        let box_rect = Rect::from_center_size(
            to_screen(self.screen_width * 0.5, selection_y),
            Vec2::new(500.0 * uniform, 70.0 * uniform),
        );
        painter.rect_stroke(
            box_rect,
            4.0 * uniform,
            egui::Stroke::new(3.0 * uniform, selected_color),
            StrokeKind::Outside,
        );

        painter.text(
            to_screen(self.screen_width * 0.5, self.screen_height - 150.0),
            egui::Align2::CENTER_CENTER,
            "Rotate to select",
            egui::FontId::proportional(28.0 * uniform),
            Color32::GRAY,
        );
        painter.text(
            to_screen(self.screen_width * 0.5, self.screen_height - 100.0),
            egui::Align2::CENTER_CENTER,
            "Press button to start",
            egui::FontId::proportional(28.0 * uniform),
            Color32::GRAY,
        );
    }

    fn render_game(&self, ui: &mut Ui) {
        let rect = ui.available_rect_before_wrap();
        let painter = ui.painter();
        let sx = rect.width() / self.screen_width;
        let sy = rect.height() / self.screen_height;
        let uniform = sx.min(sy);
        let to_screen = |gx: f32, gy: f32| Pos2::new(rect.min.x + gx * sx, rect.min.y + gy * sy);

        painter.rect_filled(rect, 0.0, Color32::BLACK);
        let white = Color32::WHITE;

        // Top wall
        painter.rect_filled(
            Rect::from_min_size(rect.min, Vec2::new(rect.width(), WALL_THICKNESS * sy)),
            0.0,
            white,
        );
        // Bottom wall
        painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.min.x, rect.max.y - WALL_THICKNESS * sy),
                Vec2::new(rect.width(), WALL_THICKNESS * sy),
            ),
            0.0,
            white,
        );

        // Center dashed line
        let center_x_screen = to_screen(self.screen_width * 0.5, 0.0).x;
        let mut y = self.play_top;
        while y < self.play_bottom {
            let top = to_screen(0.0, y).y;
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(center_x_screen - 2.0 * uniform, top),
                    Vec2::new(4.0 * uniform, CENTER_LINE_SEGMENT * sy),
                ),
                0.0,
                white,
            );
            y += CENTER_LINE_SEGMENT + CENTER_LINE_GAP;
        }

        // Left paddle
        let left_paddle_rect = Rect::from_center_size(
            to_screen(PADDLE_MARGIN + PADDLE_WIDTH * 0.5, self.left_paddle_y),
            Vec2::new(PADDLE_WIDTH * sx, PADDLE_HEIGHT * sy),
        );
        painter.rect_filled(left_paddle_rect, 0.0, white);

        // Right paddle
        let right_paddle_rect = Rect::from_center_size(
            to_screen(
                self.screen_width - PADDLE_MARGIN - PADDLE_WIDTH * 0.5,
                self.right_paddle_y,
            ),
            Vec2::new(PADDLE_WIDTH * sx, PADDLE_HEIGHT * sy),
        );
        painter.rect_filled(right_paddle_rect, 0.0, white);

        // Ball
        if !self.serving || (self.serve_timer * 4.0) as i32 % 2 == 0 {
            let ball_rect = Rect::from_center_size(
                to_screen(self.ball_x, self.ball_y),
                Vec2::new(BALL_SIZE * sx, BALL_SIZE * sy),
            );
            painter.rect_filled(ball_rect, 0.0, white);
        }

        // Scores
        let score_y = 80.0;
        painter.text(
            to_screen(self.screen_width * 0.25, score_y),
            egui::Align2::CENTER_CENTER,
            format!("{}", self.left_score),
            egui::FontId::monospace(72.0 * uniform),
            white,
        );
        painter.text(
            to_screen(self.screen_width * 0.75, score_y),
            egui::Align2::CENTER_CENTER,
            format!("{}", self.right_score),
            egui::FontId::monospace(72.0 * uniform),
            white,
        );

        // Mode indicator
        let mode_text = match self.mode {
            PongMode::OnePlayer => "1P vs AI",
            PongMode::TwoPlayer => "P1 vs P2",
        };
        painter.text(
            to_screen(self.screen_width * 0.5, 140.0),
            egui::Align2::CENTER_CENTER,
            mode_text,
            egui::FontId::proportional(24.0 * uniform),
            Color32::GRAY,
        );

        // Game over message
        if self.state == GameState::GameOver {
            let message = match self.mode {
                PongMode::OnePlayer => {
                    if self.left_won { "YOU WIN!" } else { "GAME OVER" }
                }
                PongMode::TwoPlayer => {
                    if self.left_won { "PLAYER 1 WINS!" } else { "PLAYER 2 WINS!" }
                }
            };
            painter.text(
                to_screen(self.screen_width * 0.5, self.screen_height * 0.5),
                egui::Align2::CENTER_CENTER,
                message,
                egui::FontId::monospace(64.0 * uniform),
                white,
            );
            painter.text(
                to_screen(self.screen_width * 0.5, self.screen_height * 0.5 + 80.0),
                egui::Align2::CENTER_CENTER,
                "Press button to play again",
                egui::FontId::proportional(28.0 * uniform),
                Color32::GRAY,
            );
        }

        // Controls hint
        let controls_hint = match self.mode {
            PongMode::OnePlayer => "Rotate to move paddle",
            PongMode::TwoPlayer => "Left: P1  •  Right: P2",
        };
        painter.text(
            to_screen(self.screen_width * 0.5, self.screen_height - 50.0),
            egui::Align2::CENTER_CENTER,
            controls_hint,
            egui::FontId::proportional(22.0 * uniform),
            Color32::DARK_GRAY,
        );
    }
}

impl Default for Pong {
    fn default() -> Self {
        Self::new()
    }
}

impl Game for Pong {
    fn update(&mut self, dt: f32, input: &Input) -> bool {
        if input.exit_requested || input.menu_requested {
            return true;
        }

        if self.button_debounce_timer > 0.0 {
            self.button_debounce_timer -= dt;
            self.menu_rotation_accumulator += input.rotation;
            return false;
        }

        match self.state {
            GameState::ModeSelect => self.update_menu(input),
            GameState::Playing => self.update_playing(dt, input),
            GameState::GameOver => {
                if input.action_pressed {
                    self.state = GameState::ModeSelect;
                    self.menu_selection = usize::from(self.mode != PongMode::OnePlayer);
                    self.menu_rotation_accumulator = 0.0;
                }
                false
            }
        }
    }

    fn render(&self, ui: &mut Ui) {
        match self.state {
            GameState::ModeSelect => self.render_menu(ui),
            GameState::Playing | GameState::GameOver => self.render_game(ui),
        }
    }

    fn name(&self) -> &str {
        "PONG"
    }

    fn description(&self) -> &str {
        "THE ORIGINAL 1972 CLASSIC"
    }

    fn year(&self) -> &str {
        "1972"
    }

    fn debug_stats(&self) -> Option<String> {
        Some(format!(
            "mode={:?}, score={}:{}, speed={:.0}, state={:?}",
            self.mode, self.left_score, self.right_score, self.ball_speed, self.state
        ))
    }
}
