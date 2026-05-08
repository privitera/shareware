//! Snake cartridge.
//!
//! Grid-based Snake with 1P and competitive 2P modes. Multiple food types
//! (normal, bonus, speed). Configurable wall wrapping.
//!
//! In 1P mode, the combined `rotation` input turns the snake. In 2P mode,
//! `rotation_left` controls the green (P1) snake, `rotation_right` controls
//! the blue (P2) snake.
//!
//! Designed for the 1080×1920 native rect; render scales linearly.

use std::collections::VecDeque;

use arcade_cart::{Game, Input, Rng};
use egui::{Color32, Pos2, Rect, Stroke, StrokeKind, Ui, Vec2};

const GRID_WIDTH: i32 = 27;
const GRID_HEIGHT: i32 = 42;
const CELL_SIZE: f32 = 40.0;
const TOP_MARGIN: f32 = 160.0;

const BASE_SPEED: f32 = 6.0;
const MAX_SPEED: f32 = 15.0;
const SPEED_INCREMENT: f32 = 0.3;

const INITIAL_LENGTH: usize = 4;

const POINTS_NORMAL: u32 = 10;
const POINTS_BONUS: u32 = 50;
const POINTS_SPEED: u32 = 25;

const BONUS_SPAWN_CHANCE: u32 = 15;
const BONUS_LIFETIME: f32 = 8.0;

const ROTATION_THRESHOLD: f32 = 0.4;
const MENU_ROTATION_THRESHOLD: f32 = 0.5;
const BUTTON_DEBOUNCE_COOLDOWN: f32 = 0.3;

const SNAKE_GREEN: Color32 = Color32::from_rgb(0, 158, 115);
const SNAKE_BLUE: Color32 = Color32::from_rgb(0, 114, 178);
const SNAKE_HEAD_HIGHLIGHT: Color32 = Color32::from_rgb(255, 255, 200);
const FOOD_ORANGE: Color32 = Color32::from_rgb(230, 159, 0);
const FOOD_BONUS: Color32 = Color32::from_rgb(204, 121, 167);
const FOOD_SPEED: Color32 = Color32::from_rgb(86, 180, 233);
const GRID_LINE: Color32 = Color32::from_rgb(30, 30, 40);
const BACKGROUND: Color32 = Color32::from_rgb(15, 15, 25);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    fn delta(self) -> (i32, i32) {
        match self {
            Self::Up => (0, -1),
            Self::Down => (0, 1),
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
        }
    }

    fn turn_cw(self) -> Self {
        match self {
            Self::Up => Self::Right,
            Self::Right => Self::Down,
            Self::Down => Self::Left,
            Self::Left => Self::Up,
        }
    }

    fn turn_ccw(self) -> Self {
        match self {
            Self::Up => Self::Left,
            Self::Left => Self::Down,
            Self::Down => Self::Right,
            Self::Right => Self::Up,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FoodType {
    Normal,
    Bonus,
    Speed,
}

#[derive(Debug, Clone)]
struct Food {
    x: i32,
    y: i32,
    food_type: FoodType,
    lifetime: Option<f32>,
}

#[derive(Debug, Clone)]
struct Snake {
    body: VecDeque<(i32, i32)>,
    direction: Direction,
    next_direction: Direction,
    alive: bool,
    score: u32,
    color: Color32,
    move_timer: f32,
    speed: f32,
    speed_boost_timer: f32,
    rotation_accumulator: f32,
}

impl Snake {
    fn new(start_x: i32, start_y: i32, direction: Direction, color: Color32) -> Self {
        let mut body = VecDeque::with_capacity(100);
        let (dx, dy) = direction.opposite().delta();
        for i in 0..INITIAL_LENGTH {
            body.push_back((start_x + dx * i as i32, start_y + dy * i as i32));
        }
        Self {
            body,
            direction,
            next_direction: direction,
            alive: true,
            score: 0,
            color,
            move_timer: 0.0,
            speed: BASE_SPEED,
            speed_boost_timer: 0.0,
            rotation_accumulator: 0.0,
        }
    }

    fn head(&self) -> (i32, i32) {
        *self.body.front().expect("snake body is never empty")
    }

    fn current_speed(&self) -> f32 {
        if self.speed_boost_timer > 0.0 {
            (self.speed * 1.5).min(MAX_SPEED)
        } else {
            self.speed
        }
    }

    fn handle_rotation(&mut self, rotation: f32) {
        self.rotation_accumulator += rotation;
        if self.rotation_accumulator > ROTATION_THRESHOLD {
            self.rotation_accumulator = 0.0;
            let new_dir = self.direction.turn_cw();
            if new_dir != self.direction.opposite() {
                self.next_direction = new_dir;
            }
        } else if self.rotation_accumulator < -ROTATION_THRESHOLD {
            self.rotation_accumulator = 0.0;
            let new_dir = self.direction.turn_ccw();
            if new_dir != self.direction.opposite() {
                self.next_direction = new_dir;
            }
        }
    }

    fn move_forward(&mut self, wrap: bool) -> bool {
        if !self.alive {
            return false;
        }
        if self.next_direction != self.direction.opposite() {
            self.direction = self.next_direction;
        }
        let (hx, hy) = self.head();
        let (dx, dy) = self.direction.delta();
        let mut new_x = hx + dx;
        let mut new_y = hy + dy;

        if wrap {
            if new_x < 0 { new_x = GRID_WIDTH - 1; }
            if new_x >= GRID_WIDTH { new_x = 0; }
            if new_y < 0 { new_y = GRID_HEIGHT - 1; }
            if new_y >= GRID_HEIGHT { new_y = 0; }
        } else if !(0..GRID_WIDTH).contains(&new_x) || !(0..GRID_HEIGHT).contains(&new_y) {
            self.alive = false;
            return false;
        }

        self.body.push_front((new_x, new_y));
        true
    }

    fn shrink_tail(&mut self) {
        self.body.pop_back();
    }

    fn self_collision(&self) -> bool {
        let head = self.head();
        self.body.iter().skip(1).any(|&seg| seg == head)
    }

    fn collides_with(&self, other: &Snake) -> bool {
        let head = self.head();
        other.body.iter().any(|&seg| seg == head)
    }

    fn occupies(&self, x: i32, y: i32) -> bool {
        self.body.iter().any(|&(sx, sy)| sx == x && sy == y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SnakeMode {
    OnePlayer,
    TwoPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameState {
    ModeSelect,
    Playing,
    GameOver,
}

pub struct SnakeGame {
    state: GameState,
    mode: SnakeMode,
    menu_selection: usize,
    menu_rotation_accumulator: f32,
    button_debounce_timer: f32,

    snake1: Snake,
    snake2: Option<Snake>,

    foods: Vec<Food>,

    rng: Rng,

    screen_width: f32,
    screen_height: f32,

    wrap_walls: bool,
}

impl SnakeGame {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: GameState::ModeSelect,
            mode: SnakeMode::OnePlayer,
            menu_selection: 0,
            menu_rotation_accumulator: 0.0,
            button_debounce_timer: BUTTON_DEBOUNCE_COOLDOWN,
            snake1: Snake::new(GRID_WIDTH / 4, GRID_HEIGHT / 2, Direction::Right, SNAKE_GREEN),
            snake2: None,
            foods: Vec::new(),
            rng: Rng::new(0xDEAD_BEEF),
            screen_width: arcade_cart::DESIGN_WIDTH,
            screen_height: arcade_cart::DESIGN_HEIGHT,
            wrap_walls: true,
        }
    }

    fn start_game(&mut self) {
        self.snake1 = Snake::new(GRID_WIDTH / 4, GRID_HEIGHT / 2, Direction::Right, SNAKE_GREEN);
        self.snake2 = if self.mode == SnakeMode::TwoPlayer {
            Some(Snake::new(GRID_WIDTH * 3 / 4, GRID_HEIGHT / 2, Direction::Left, SNAKE_BLUE))
        } else {
            None
        };

        self.foods.clear();
        self.spawn_food(FoodType::Normal);
        self.spawn_food(FoodType::Normal);
        if self.mode == SnakeMode::TwoPlayer {
            self.spawn_food(FoodType::Normal);
        }

        self.state = GameState::Playing;
    }

    fn spawn_food(&mut self, food_type: FoodType) {
        for _ in 0..100 {
            let x = self.rng.range(GRID_WIDTH as u32) as i32;
            let y = self.rng.range(GRID_HEIGHT as u32) as i32;

            let occupied = self.snake1.occupies(x, y)
                || self.snake2.as_ref().is_some_and(|s| s.occupies(x, y))
                || self.foods.iter().any(|f| f.x == x && f.y == y);

            if !occupied {
                let lifetime = match food_type {
                    FoodType::Normal => None,
                    FoodType::Bonus | FoodType::Speed => Some(BONUS_LIFETIME),
                };
                self.foods.push(Food { x, y, food_type, lifetime });
                return;
            }
        }
    }

    fn update_menu(&mut self, input: &Input) {
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
                SnakeMode::OnePlayer
            } else {
                SnakeMode::TwoPlayer
            };
            self.start_game();
        }
    }

    fn update_playing(&mut self, dt: f32, input: &Input) {
        self.foods.retain_mut(|food| {
            if let Some(ref mut lifetime) = food.lifetime {
                *lifetime -= dt;
                *lifetime > 0.0
            } else {
                true
            }
        });

        if !self.foods.iter().any(|f| f.food_type == FoodType::Normal) {
            self.spawn_food(FoodType::Normal);
        }

        if self.snake1.speed_boost_timer > 0.0 {
            self.snake1.speed_boost_timer -= dt;
        }
        if let Some(ref mut snake2) = self.snake2
            && snake2.speed_boost_timer > 0.0
        {
            snake2.speed_boost_timer -= dt;
        }

        match self.mode {
            SnakeMode::OnePlayer => {
                self.snake1.handle_rotation(input.rotation);
            }
            SnakeMode::TwoPlayer => {
                self.snake1.handle_rotation(input.rotation_left);
                if let Some(ref mut snake2) = self.snake2 {
                    snake2.handle_rotation(input.rotation_right);
                }
            }
        }

        let mut spawn_normal = 0;
        let mut spawn_bonus_count = 0;
        let mut spawn_speed_count = 0;

        if self.snake1.alive {
            self.snake1.move_timer += dt;
            let move_interval = 1.0 / self.snake1.current_speed();

            while self.snake1.move_timer >= move_interval {
                self.snake1.move_timer -= move_interval;

                if self.snake1.move_forward(self.wrap_walls) {
                    if self.snake1.self_collision() {
                        self.snake1.alive = false;
                    }
                    if let Some(ref snake2) = self.snake2
                        && self.snake1.collides_with(snake2)
                    {
                        self.snake1.alive = false;
                    }

                    let head = self.snake1.head();
                    let mut ate_food = false;
                    let mut eaten_type = None;

                    if let Some(idx) = self.foods.iter().position(|f| f.x == head.0 && f.y == head.1)
                    {
                        eaten_type = Some(self.foods[idx].food_type);
                        self.foods.remove(idx);
                        ate_food = true;
                    }

                    if let Some(food_type) = eaten_type {
                        match food_type {
                            FoodType::Normal => {
                                self.snake1.score += POINTS_NORMAL;
                                self.snake1.speed = (self.snake1.speed + SPEED_INCREMENT).min(MAX_SPEED);
                                spawn_normal += 1;
                                if self.rng.range(100) < BONUS_SPAWN_CHANCE {
                                    if self.rng.range(2) == 0 {
                                        spawn_bonus_count += 1;
                                    } else {
                                        spawn_speed_count += 1;
                                    }
                                }
                            }
                            FoodType::Bonus => self.snake1.score += POINTS_BONUS,
                            FoodType::Speed => {
                                self.snake1.score += POINTS_SPEED;
                                self.snake1.speed_boost_timer = 5.0;
                            }
                        }
                    }

                    if !ate_food {
                        self.snake1.shrink_tail();
                    }
                }
            }
        }

        if let Some(ref mut snake2) = self.snake2
            && snake2.alive
        {
            let move_interval = 1.0 / snake2.current_speed();
            let wrap = self.wrap_walls;
            snake2.move_timer += dt;

            while snake2.move_timer >= move_interval {
                snake2.move_timer -= move_interval;

                if snake2.move_forward(wrap) {
                    if snake2.self_collision() {
                        snake2.alive = false;
                    }
                    if snake2.collides_with(&self.snake1) {
                        snake2.alive = false;
                    }

                    let head = snake2.head();
                    let mut ate_food = false;
                    let mut eaten_type = None;

                    if let Some(idx) = self.foods.iter().position(|f| f.x == head.0 && f.y == head.1)
                    {
                        eaten_type = Some(self.foods[idx].food_type);
                        self.foods.remove(idx);
                        ate_food = true;
                    }

                    if let Some(food_type) = eaten_type {
                        match food_type {
                            FoodType::Normal => {
                                snake2.score += POINTS_NORMAL;
                                snake2.speed = (snake2.speed + SPEED_INCREMENT).min(MAX_SPEED);
                                spawn_normal += 1;
                                spawn_bonus_count += 1;
                            }
                            FoodType::Bonus => snake2.score += POINTS_BONUS,
                            FoodType::Speed => {
                                snake2.score += POINTS_SPEED;
                                snake2.speed_boost_timer = 5.0;
                            }
                        }
                    }

                    if !ate_food {
                        snake2.shrink_tail();
                    }
                }
            }
        }

        for _ in 0..spawn_normal {
            self.spawn_food(FoodType::Normal);
        }
        for _ in 0..spawn_bonus_count {
            self.spawn_food(FoodType::Bonus);
        }
        for _ in 0..spawn_speed_count {
            self.spawn_food(FoodType::Speed);
        }

        if let Some(ref mut snake2) = self.snake2
            && self.snake1.alive
            && snake2.alive
            && self.snake1.head() == snake2.head()
        {
            self.snake1.alive = false;
            snake2.alive = false;
        }

        let game_over = match self.mode {
            SnakeMode::OnePlayer => !self.snake1.alive,
            SnakeMode::TwoPlayer => {
                !self.snake1.alive || !self.snake2.as_ref().is_none_or(|s| s.alive)
            }
        };

        if game_over {
            self.state = GameState::GameOver;
        }
    }

    fn render_menu(&self, ui: &mut Ui) {
        let rect = ui.available_rect_before_wrap();
        let painter = ui.painter();
        let center_x = rect.center().x;
        let sx = rect.width() / self.screen_width;
        let sy = rect.height() / self.screen_height;
        let uniform = sx.min(sy);

        painter.rect_filled(rect, 0.0, BACKGROUND);

        // Subtle grid (entire rect)
        let mut x_g = 0.0;
        while x_g <= self.screen_width {
            let px = rect.min.x + x_g * sx;
            painter.line_segment(
                [Pos2::new(px, rect.min.y), Pos2::new(px, rect.max.y)],
                Stroke::new(1.0, GRID_LINE),
            );
            x_g += CELL_SIZE;
        }
        let mut y_g = 0.0;
        while y_g <= self.screen_height {
            let py = rect.min.y + y_g * sy;
            painter.line_segment(
                [Pos2::new(rect.min.x, py), Pos2::new(rect.max.x, py)],
                Stroke::new(1.0, GRID_LINE),
            );
            y_g += CELL_SIZE;
        }

        // Title
        painter.text(
            Pos2::new(center_x, rect.min.y + 250.0 * sy),
            egui::Align2::CENTER_CENTER,
            "SNAKE",
            egui::FontId::monospace(96.0 * uniform),
            SNAKE_GREEN,
        );

        // Decorative snake
        let snake_y = rect.min.y + 380.0 * sy;
        for i in 0..8 {
            let x = center_x - 140.0 * uniform + i as f32 * 40.0 * uniform;
            let wave = (i as f32 * 0.5).sin() * 10.0 * uniform;
            let size = if i == 7 { 36.0 * uniform } else { 32.0 * uniform };
            let color = if i == 7 { SNAKE_HEAD_HIGHLIGHT } else { SNAKE_GREEN };
            painter.rect_filled(
                Rect::from_center_size(Pos2::new(x, snake_y + wave), Vec2::splat(size)),
                4.0 * uniform,
                color,
            );
        }

        let option_y_1p = rect.min.y + 550.0 * sy;
        let option_y_2p = rect.min.y + 700.0 * sy;
        let selected_color = SNAKE_GREEN;
        let white = Color32::WHITE;

        let color_1p = if self.menu_selection == 0 { selected_color } else { white };
        let prefix_1p = if self.menu_selection == 0 { ">" } else { " " };
        painter.text(
            Pos2::new(center_x, option_y_1p),
            egui::Align2::CENTER_CENTER,
            format!("{prefix_1p} 1 PLAYER"),
            egui::FontId::monospace(48.0 * uniform),
            color_1p,
        );
        painter.text(
            Pos2::new(center_x, option_y_1p + 50.0 * uniform),
            egui::Align2::CENTER_CENTER,
            "Classic solo mode",
            egui::FontId::proportional(24.0 * uniform),
            Color32::GRAY,
        );

        let color_2p = if self.menu_selection == 1 { selected_color } else { white };
        let prefix_2p = if self.menu_selection == 1 { ">" } else { " " };
        painter.text(
            Pos2::new(center_x, option_y_2p),
            egui::Align2::CENTER_CENTER,
            format!("{prefix_2p} 2 PLAYERS"),
            egui::FontId::monospace(48.0 * uniform),
            color_2p,
        );
        painter.text(
            Pos2::new(center_x, option_y_2p + 50.0 * uniform),
            egui::Align2::CENTER_CENTER,
            "Last snake wins",
            egui::FontId::proportional(24.0 * uniform),
            Color32::GRAY,
        );

        let selection_y = if self.menu_selection == 0 { option_y_1p } else { option_y_2p };
        let box_rect = Rect::from_center_size(
            Pos2::new(center_x, selection_y),
            Vec2::new(450.0 * uniform, 70.0 * uniform),
        );
        painter.rect_stroke(
            box_rect,
            4.0 * uniform,
            Stroke::new(3.0 * uniform, selected_color),
            StrokeKind::Outside,
        );

        painter.text(
            Pos2::new(center_x, rect.max.y - 200.0 * uniform),
            egui::Align2::CENTER_CENTER,
            "Rotate to turn",
            egui::FontId::proportional(26.0 * uniform),
            Color32::GRAY,
        );
        painter.text(
            Pos2::new(center_x, rect.max.y - 150.0 * uniform),
            egui::Align2::CENTER_CENTER,
            "Press button to start",
            egui::FontId::proportional(26.0 * uniform),
            Color32::GRAY,
        );
    }

    fn render_game(&self, ui: &mut Ui) {
        let rect = ui.available_rect_before_wrap();
        let painter = ui.painter();
        let center_x = rect.center().x;
        let sx = rect.width() / self.screen_width;
        let sy = rect.height() / self.screen_height;
        let uniform = sx.min(sy);

        painter.rect_filled(rect, 0.0, BACKGROUND);

        let game_left = (self.screen_width - GRID_WIDTH as f32 * CELL_SIZE) * 0.5;
        let game_top = TOP_MARGIN;

        for x in 0..=GRID_WIDTH {
            let px = rect.min.x + (game_left + x as f32 * CELL_SIZE) * sx;
            painter.line_segment(
                [
                    Pos2::new(px, rect.min.y + game_top * sy),
                    Pos2::new(px, rect.min.y + (game_top + GRID_HEIGHT as f32 * CELL_SIZE) * sy),
                ],
                Stroke::new(1.0, GRID_LINE),
            );
        }
        for y in 0..=GRID_HEIGHT {
            let py = rect.min.y + (game_top + y as f32 * CELL_SIZE) * sy;
            painter.line_segment(
                [
                    Pos2::new(rect.min.x + game_left * sx, py),
                    Pos2::new(rect.min.x + (game_left + GRID_WIDTH as f32 * CELL_SIZE) * sx, py),
                ],
                Stroke::new(1.0, GRID_LINE),
            );
        }

        let border_rect = Rect::from_min_size(
            Pos2::new(rect.min.x + game_left * sx, rect.min.y + game_top * sy),
            Vec2::new(GRID_WIDTH as f32 * CELL_SIZE * sx, GRID_HEIGHT as f32 * CELL_SIZE * sy),
        );
        painter.rect_stroke(
            border_rect,
            0.0,
            Stroke::new(3.0 * uniform, Color32::from_rgb(60, 60, 80)),
            StrokeKind::Inside,
        );

        let cell_w = CELL_SIZE * sx;
        let cell_h = CELL_SIZE * sy;

        let draw_cell = |x: i32, y: i32, color: Color32, shrink: f32| {
            let px = rect.min.x + (game_left + x as f32 * CELL_SIZE) * sx + shrink;
            let py = rect.min.y + (game_top + y as f32 * CELL_SIZE) * sy + shrink;
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(px, py),
                    Vec2::new(cell_w - shrink * 2.0, cell_h - shrink * 2.0),
                ),
                4.0 * uniform,
                color,
            );
        };

        for food in &self.foods {
            let color = match food.food_type {
                FoodType::Normal => FOOD_ORANGE,
                FoodType::Bonus => FOOD_BONUS,
                FoodType::Speed => FOOD_SPEED,
            };
            let shrink = if food.lifetime.is_some() {
                (2.0 + (food.lifetime.unwrap_or(0.0) * 8.0).sin().abs() * 3.0) * uniform
            } else {
                2.0 * uniform
            };
            draw_cell(food.x, food.y, color, shrink);

            let fx = rect.min.x + (game_left + food.x as f32 * CELL_SIZE + CELL_SIZE * 0.5) * sx;
            let fy = rect.min.y + (game_top + food.y as f32 * CELL_SIZE + CELL_SIZE * 0.5) * sy;
            let symbol = match food.food_type {
                FoodType::Normal => "●",
                FoodType::Bonus => "★",
                FoodType::Speed => "»",
            };
            painter.text(
                Pos2::new(fx, fy),
                egui::Align2::CENTER_CENTER,
                symbol,
                egui::FontId::proportional(24.0 * uniform),
                Color32::WHITE,
            );
        }

        let draw_snake = |snake: &Snake| {
            for (i, &(x, y)) in snake.body.iter().enumerate() {
                let is_head = i == 0;
                let alpha = if snake.alive { 255 } else { 128 };
                let color = if is_head {
                    Color32::from_rgba_unmultiplied(
                        SNAKE_HEAD_HIGHLIGHT.r(),
                        SNAKE_HEAD_HIGHLIGHT.g(),
                        SNAKE_HEAD_HIGHLIGHT.b(),
                        alpha,
                    )
                } else {
                    Color32::from_rgba_unmultiplied(
                        snake.color.r(),
                        snake.color.g(),
                        snake.color.b(),
                        alpha,
                    )
                };
                let shrink = if is_head { 1.0 * uniform } else { 2.0 * uniform };
                draw_cell(x, y, color, shrink);

                if is_head && snake.alive {
                    let hx = rect.min.x + (game_left + x as f32 * CELL_SIZE + CELL_SIZE * 0.5) * sx;
                    let hy = rect.min.y + (game_top + y as f32 * CELL_SIZE + CELL_SIZE * 0.5) * sy;
                    let (ex, ey) = match snake.direction {
                        Direction::Up => (0.0, -8.0 * uniform),
                        Direction::Down => (0.0, 8.0 * uniform),
                        Direction::Left => (-8.0 * uniform, 0.0),
                        Direction::Right => (8.0 * uniform, 0.0),
                    };
                    painter.circle_filled(
                        Pos2::new(hx + ex - 5.0 * uniform, hy + ey),
                        4.0 * uniform,
                        Color32::BLACK,
                    );
                    painter.circle_filled(
                        Pos2::new(hx + ex + 5.0 * uniform, hy + ey),
                        4.0 * uniform,
                        Color32::BLACK,
                    );
                }
            }
        };

        draw_snake(&self.snake1);
        if let Some(ref snake2) = self.snake2 {
            draw_snake(snake2);
        }

        // Scores
        match self.mode {
            SnakeMode::OnePlayer => {
                painter.text(
                    Pos2::new(center_x, rect.min.y + 50.0 * uniform),
                    egui::Align2::CENTER_CENTER,
                    format!("SCORE: {}", self.snake1.score),
                    egui::FontId::monospace(48.0 * uniform),
                    Color32::WHITE,
                );
                painter.text(
                    Pos2::new(center_x, rect.min.y + 100.0 * uniform),
                    egui::Align2::CENTER_CENTER,
                    format!("Length: {}", self.snake1.body.len()),
                    egui::FontId::proportional(24.0 * uniform),
                    Color32::GRAY,
                );
            }
            SnakeMode::TwoPlayer => {
                painter.text(
                    Pos2::new(rect.min.x + 150.0 * uniform, rect.min.y + 50.0 * uniform),
                    egui::Align2::CENTER_CENTER,
                    format!("P1: {}", self.snake1.score),
                    egui::FontId::monospace(40.0 * uniform),
                    SNAKE_GREEN,
                );
                if let Some(ref snake2) = self.snake2 {
                    painter.text(
                        Pos2::new(rect.max.x - 150.0 * uniform, rect.min.y + 50.0 * uniform),
                        egui::Align2::CENTER_CENTER,
                        format!("P2: {}", snake2.score),
                        egui::FontId::monospace(40.0 * uniform),
                        SNAKE_BLUE,
                    );
                }
                painter.text(
                    Pos2::new(rect.min.x + 150.0 * uniform, rect.min.y + 95.0 * uniform),
                    egui::Align2::CENTER_CENTER,
                    format!("Len: {}", self.snake1.body.len()),
                    egui::FontId::proportional(20.0 * uniform),
                    Color32::GRAY,
                );
                if let Some(ref snake2) = self.snake2 {
                    painter.text(
                        Pos2::new(rect.max.x - 150.0 * uniform, rect.min.y + 95.0 * uniform),
                        egui::Align2::CENTER_CENTER,
                        format!("Len: {}", snake2.body.len()),
                        egui::FontId::proportional(20.0 * uniform),
                        Color32::GRAY,
                    );
                }
            }
        }

        if self.snake1.speed_boost_timer > 0.0 {
            painter.text(
                Pos2::new(rect.min.x + 150.0 * uniform, rect.min.y + 130.0 * uniform),
                egui::Align2::CENTER_CENTER,
                format!("BOOST {:.1}s", self.snake1.speed_boost_timer),
                egui::FontId::proportional(20.0 * uniform),
                FOOD_SPEED,
            );
        }
        if let Some(ref snake2) = self.snake2
            && snake2.speed_boost_timer > 0.0
        {
            painter.text(
                Pos2::new(rect.max.x - 150.0 * uniform, rect.min.y + 130.0 * uniform),
                egui::Align2::CENTER_CENTER,
                format!("BOOST {:.1}s", snake2.speed_boost_timer),
                egui::FontId::proportional(20.0 * uniform),
                FOOD_SPEED,
            );
        }

        if self.state == GameState::GameOver {
            painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, 180));

            match self.mode {
                SnakeMode::OnePlayer => {
                    painter.text(
                        Pos2::new(center_x, rect.min.y + 400.0 * uniform),
                        egui::Align2::CENTER_CENTER,
                        "GAME OVER",
                        egui::FontId::monospace(72.0 * uniform),
                        Color32::from_rgb(213, 94, 0),
                    );
                    painter.text(
                        Pos2::new(center_x, rect.min.y + 500.0 * uniform),
                        egui::Align2::CENTER_CENTER,
                        format!("Final Score: {}", self.snake1.score),
                        egui::FontId::monospace(48.0 * uniform),
                        Color32::WHITE,
                    );
                    painter.text(
                        Pos2::new(center_x, rect.min.y + 570.0 * uniform),
                        egui::Align2::CENTER_CENTER,
                        format!("Length: {}", self.snake1.body.len()),
                        egui::FontId::proportional(32.0 * uniform),
                        Color32::GRAY,
                    );
                }
                SnakeMode::TwoPlayer => {
                    let winner = if !self.snake1.alive
                        && self.snake2.as_ref().is_none_or(|s| s.alive)
                    {
                        Some(("PLAYER 2 WINS!", SNAKE_BLUE))
                    } else if self.snake1.alive
                        && !self.snake2.as_ref().is_none_or(|s| s.alive)
                    {
                        Some(("PLAYER 1 WINS!", SNAKE_GREEN))
                    } else {
                        None
                    };

                    if let Some((text, color)) = winner {
                        painter.text(
                            Pos2::new(center_x, rect.min.y + 400.0 * uniform),
                            egui::Align2::CENTER_CENTER,
                            text,
                            egui::FontId::monospace(64.0 * uniform),
                            color,
                        );
                    } else {
                        painter.text(
                            Pos2::new(center_x, rect.min.y + 400.0 * uniform),
                            egui::Align2::CENTER_CENTER,
                            "DRAW!",
                            egui::FontId::monospace(72.0 * uniform),
                            Color32::YELLOW,
                        );
                    }

                    painter.text(
                        Pos2::new(center_x, rect.min.y + 500.0 * uniform),
                        egui::Align2::CENTER_CENTER,
                        format!(
                            "P1: {} pts ({} len)  |  P2: {} pts ({} len)",
                            self.snake1.score,
                            self.snake1.body.len(),
                            self.snake2.as_ref().map_or(0, |s| s.score),
                            self.snake2.as_ref().map_or(0, |s| s.body.len())
                        ),
                        egui::FontId::proportional(28.0 * uniform),
                        Color32::WHITE,
                    );
                }
            }

            painter.text(
                Pos2::new(center_x, rect.min.y + 700.0 * uniform),
                egui::Align2::CENTER_CENTER,
                "Press button to play again",
                egui::FontId::proportional(28.0 * uniform),
                Color32::GRAY,
            );
        }

        if self.state == GameState::Playing {
            let hint = match self.mode {
                SnakeMode::OnePlayer => "Rotate to turn",
                SnakeMode::TwoPlayer => "Left: P1  •  Right: P2",
            };
            painter.text(
                Pos2::new(center_x, rect.max.y - 50.0 * uniform),
                egui::Align2::CENTER_CENTER,
                hint,
                egui::FontId::proportional(22.0 * uniform),
                Color32::DARK_GRAY,
            );
        }
    }
}

impl Default for SnakeGame {
    fn default() -> Self {
        Self::new()
    }
}

impl Game for SnakeGame {
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
                    self.menu_selection = usize::from(self.mode != SnakeMode::OnePlayer);
                    self.menu_rotation_accumulator = 0.0;
                }
            }
        }

        false
    }

    fn render(&self, ui: &mut Ui) {
        match self.state {
            GameState::ModeSelect => self.render_menu(ui),
            GameState::Playing | GameState::GameOver => self.render_game(ui),
        }
    }

    fn name(&self) -> &str {
        "SNAKE"
    }

    fn description(&self) -> &str {
        "EAT, GROW, SURVIVE"
    }

    fn year(&self) -> &str {
        "1976"
    }

    fn debug_stats(&self) -> Option<String> {
        Some(format!(
            "mode={:?}, p1_score={}, p1_len={}, state={:?}",
            self.mode,
            self.snake1.score,
            self.snake1.body.len(),
            self.state
        ))
    }
}
