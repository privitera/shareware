//! SkiFree cartridge.
//!
//! Rust port of the 1991 Windows game by Chris Pirih. Sprite art is sourced
//! from [`basicallydan/skifree.js`](https://github.com/basicallydan/skifree.js)
//! by Dan Hough (MIT). See `NOTICE`.
//!
//! Mechanics preserved from the original:
//!
//! - 7 discrete skiing directions with specific (x, y) speed factors
//! - Obstacle drop rates tuned to match the original feel on a portrait viewport
//! - Crash recovery timing (1.5s)
//! - Distance tracking in metres (18 px = 1 m)
//! - Yeti chase begins after 2000 m
//!
//! ## Coordinate space
//!
//! Game logic runs in a 1080×1920 design coordinate space (matches
//! [`arcade_cart::DESIGN_WIDTH`] / [`arcade_cart::DESIGN_HEIGHT`]). The render
//! method scales linearly to fit whatever rect the host provides — embedding
//! at native size is identity-scale; standalone window scales down uniformly.

use arcade_cart::{Game, Input, Rng};
use egui::{Color32, Image, Pos2, Rect, TextureOptions, Ui, Vec2};

// === SPRITE SHEET CONSTANTS ===
// sprite-characters.png: 273×583 pixels
const CHAR_SHEET_WIDTH: f32 = 273.0;
const CHAR_SHEET_HEIGHT: f32 = 583.0;

// skifree-objects.png: 337×283 pixels
const OBJ_SHEET_WIDTH: f32 = 337.0;
const OBJ_SHEET_HEIGHT: f32 = 283.0;

/// Sprite region in pixel coordinates [x, y, width, height], matching
/// `spriteInfo.js` from `skifree.js` exactly.
#[derive(Clone, Copy)]
struct SpriteRegion {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl SpriteRegion {
    const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    fn uv(self, sheet_w: f32, sheet_h: f32) -> Rect {
        Rect::from_min_max(
            Pos2::new(self.x / sheet_w, self.y / sheet_h),
            Pos2::new((self.x + self.w) / sheet_w, (self.y + self.h) / sheet_h),
        )
    }
}

// === SKIER SPRITES (from sprite-characters.png) ===
const SKIER_EAST: SpriteRegion = SpriteRegion::new(0.0, 0.0, 24.0, 34.0);
const SKIER_ES_EAST: SpriteRegion = SpriteRegion::new(24.0, 0.0, 24.0, 34.0);
const SKIER_SE_EAST: SpriteRegion = SpriteRegion::new(49.0, 0.0, 17.0, 34.0);
const SKIER_SOUTH: SpriteRegion = SpriteRegion::new(65.0, 0.0, 17.0, 34.0);
const SKIER_SW_EST: SpriteRegion = SpriteRegion::new(49.0, 37.0, 17.0, 34.0);
const SKIER_WS_WEST: SpriteRegion = SpriteRegion::new(24.0, 37.0, 24.0, 34.0);
const SKIER_WEST: SpriteRegion = SpriteRegion::new(0.0, 37.0, 24.0, 34.0);
const SKIER_HIT: SpriteRegion = SpriteRegion::new(0.0, 78.0, 31.0, 31.0);
const SKIER_JUMPING: SpriteRegion = SpriteRegion::new(84.0, 0.0, 32.0, 34.0);

// === MONSTER (YETI) SPRITES ===
const MONSTER_EAST_1: SpriteRegion = SpriteRegion::new(64.0, 112.0, 26.0, 43.0);
const MONSTER_EAST_2: SpriteRegion = SpriteRegion::new(90.0, 112.0, 32.0, 43.0);
const MONSTER_WEST_1: SpriteRegion = SpriteRegion::new(64.0, 158.0, 26.0, 43.0);
const MONSTER_WEST_2: SpriteRegion = SpriteRegion::new(90.0, 158.0, 32.0, 43.0);
const MONSTER_EATING_1: SpriteRegion = SpriteRegion::new(122.0, 112.0, 34.0, 43.0);
const MONSTER_EATING_2: SpriteRegion = SpriteRegion::new(156.0, 112.0, 31.0, 43.0);
const MONSTER_EATING_3: SpriteRegion = SpriteRegion::new(187.0, 112.0, 31.0, 43.0);
const MONSTER_EATING_4: SpriteRegion = SpriteRegion::new(219.0, 112.0, 25.0, 43.0);
const MONSTER_EATING_5: SpriteRegion = SpriteRegion::new(243.0, 112.0, 26.0, 43.0);

// === OBSTACLE SPRITES ===
const SMALL_TREE: SpriteRegion = SpriteRegion::new(0.0, 28.0, 30.0, 34.0);
const TALL_TREE: SpriteRegion = SpriteRegion::new(95.0, 66.0, 32.0, 64.0);
const ROCK: SpriteRegion = SpriteRegion::new(30.0, 52.0, 23.0, 11.0);
const JUMP_RAMP: SpriteRegion = SpriteRegion::new(109.0, 55.0, 32.0, 8.0);

// === ORIGINAL GAME CONSTANTS ===

const PIXELS_PER_METRE: f32 = 18.0;

/// Original JS: 5 px/cycle at 50 FPS = 250 px/s. With 2× sprite zoom we run
/// at ~500 px/s for the same feel; slightly reduced for controllability.
const STANDARD_SPEED: f32 = 400.0;
const CRASH_RECOVERY_TIME: f32 = 1.5;
const TURN_THRESHOLD: f32 = 0.4;
const SPAWN_CHECK_INTERVAL: f32 = 0.02;
const MAX_OBSTACLES: usize = 150;
const MONSTER_DISTANCE_THRESHOLD: f32 = 2000.0;
const MONSTER_SPAWN_RATE: u32 = 1;
const MONSTER_SPEED: f32 = 400.0;
const EATING_FRAME_TIME: f32 = 0.3;
const EATING_FRAMES: usize = 5;
const BOOST_MULTIPLIER: f32 = 2.0;
const BOOST_DURATION: f32 = 2.0;
const BOOST_COOLDOWN: f32 = 10.0;
const STARTING_LIVES: u32 = 5;

// Drop rates per spawn check, out of 1000.
const DROP_RATE_SMALL_TREE: u32 = 50;
const DROP_RATE_TALL_TREE: u32 = 25;
const DROP_RATE_ROCK: u32 = 15;
const DROP_RATE_JUMP: u32 = 8;

/// The 7 discrete skiing directions from original SkiFree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkierDirection {
    West,
    WsWest,
    SWest,
    South,
    SEast,
    EsEast,
    East,
}

impl SkierDirection {
    fn speed_factors(self) -> (f32, f32) {
        match self {
            Self::West | Self::East => (0.0, 0.0),
            Self::WsWest => (-0.5, 0.6),
            Self::EsEast => (0.5, 0.6),
            Self::SWest => (-0.33, 0.85),
            Self::SEast => (0.33, 0.85),
            Self::South => (0.0, 1.0),
        }
    }

    fn sprite_region(self) -> SpriteRegion {
        match self {
            Self::West => SKIER_WEST,
            Self::WsWest => SKIER_WS_WEST,
            Self::SWest => SKIER_SW_EST,
            Self::South => SKIER_SOUTH,
            Self::SEast => SKIER_SE_EAST,
            Self::EsEast => SKIER_ES_EAST,
            Self::East => SKIER_EAST,
        }
    }

    fn is_stopped(self) -> bool {
        matches!(self, Self::West | Self::East)
    }

    fn turn_left(self) -> Self {
        match self {
            Self::East => Self::EsEast,
            Self::EsEast => Self::SEast,
            Self::SEast => Self::South,
            Self::South => Self::SWest,
            Self::SWest => Self::WsWest,
            Self::WsWest => Self::West,
            Self::West => Self::West,
        }
    }

    fn turn_right(self) -> Self {
        match self {
            Self::West => Self::WsWest,
            Self::WsWest => Self::SWest,
            Self::SWest => Self::South,
            Self::South => Self::SEast,
            Self::SEast => Self::EsEast,
            Self::EsEast => Self::East,
            Self::East => Self::East,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObstacleType {
    SmallTree,
    TallTree,
    Rock,
    Jump,
}

impl ObstacleType {
    fn collision_width(self) -> f32 {
        match self {
            Self::SmallTree => 20.0,
            Self::TallTree => 24.0,
            Self::Rock => 18.0,
            Self::Jump => 28.0,
        }
    }

    fn collision_height(self) -> f32 {
        match self {
            Self::SmallTree => 16.0,
            Self::TallTree => 16.0,
            Self::Rock => 10.0,
            Self::Jump => 8.0,
        }
    }

    fn sprite_region(self) -> SpriteRegion {
        match self {
            Self::SmallTree => SMALL_TREE,
            Self::TallTree => TALL_TREE,
            Self::Rock => ROCK,
            Self::Jump => JUMP_RAMP,
        }
    }
}

struct Obstacle {
    x: f32,
    y: f32,
    kind: ObstacleType,
    hit: bool,
}

/// The abominable snowman. Activates after [`MONSTER_DISTANCE_THRESHOLD`] m.
struct Monster {
    x: f32,
    y: f32,
    animation_timer: f32,
    animation_frame: u8,
    eating: bool,
    eating_frame: usize,
    eating_timer: f32,
    active: bool,
}

impl Monster {
    fn new(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            animation_timer: 0.0,
            animation_frame: 1,
            eating: false,
            eating_frame: 0,
            eating_timer: 0.0,
            active: true,
        }
    }

    fn sprite(&self, moving_east: bool) -> SpriteRegion {
        if self.eating {
            match self.eating_frame {
                0 => MONSTER_EATING_1,
                1 => MONSTER_EATING_2,
                2 => MONSTER_EATING_3,
                3 => MONSTER_EATING_4,
                _ => MONSTER_EATING_5,
            }
        } else if moving_east {
            if self.animation_frame == 1 {
                MONSTER_EAST_1
            } else {
                MONSTER_EAST_2
            }
        } else if self.animation_frame == 1 {
            MONSTER_WEST_1
        } else {
            MONSTER_WEST_2
        }
    }

    fn update(&mut self, dt: f32, target_x: f32, target_y: f32) -> bool {
        if !self.active {
            return false;
        }

        if self.eating {
            self.eating_timer -= dt;
            if self.eating_timer <= 0.0 {
                self.eating_frame += 1;
                self.eating_timer = EATING_FRAME_TIME;
                if self.eating_frame >= EATING_FRAMES {
                    self.active = false;
                    return true;
                }
            }
            return false;
        }

        self.animation_timer += dt;
        if self.animation_timer > 0.1 {
            self.animation_timer = 0.0;
            self.animation_frame = if self.animation_frame == 1 { 2 } else { 1 };
        }

        let dx = target_x - self.x;
        let dy = target_y - self.y;
        let dist = (dx * dx + dy * dy).sqrt();
        if dist > 1.0 {
            let speed = MONSTER_SPEED * dt;
            self.x += (dx / dist) * speed;
            self.y += (dy / dist) * speed;
        }

        false
    }

    fn start_eating(&mut self) {
        self.eating = true;
        self.eating_frame = 0;
        self.eating_timer = EATING_FRAME_TIME;
    }

    fn caught_skier(&self, skier_x: f32, skier_y: f32) -> bool {
        if self.eating || !self.active {
            return false;
        }
        let dx = (self.x - skier_x).abs();
        let dy = (self.y - skier_y).abs();
        dx < 30.0 && dy < 30.0
    }
}

pub struct SkiFree {
    skier_x: f32,
    direction: SkierDirection,

    world_y: f32,
    obstacles: Vec<Obstacle>,

    monster: Option<Monster>,
    being_eaten: bool,

    distance_metres: f32,
    crashed: bool,
    crash_timer: f32,
    jumping: bool,
    jump_timer: f32,
    game_over: bool,
    lives: u32,

    boosting: bool,
    boost_timer: f32,
    boost_cooldown: f32,

    spawn_timer: f32,
    rng: Rng,

    rotation_accumulator: f32,

    // Game-coord screen size — locked to arcade-cart's design rect.
    screen_width: f32,
    screen_height: f32,
}

impl SkiFree {
    #[must_use]
    pub fn new() -> Self {
        Self {
            skier_x: arcade_cart::DESIGN_WIDTH * 0.5,
            direction: SkierDirection::South,
            world_y: 0.0,
            obstacles: Vec::with_capacity(MAX_OBSTACLES),
            monster: None,
            being_eaten: false,
            distance_metres: 0.0,
            crashed: false,
            crash_timer: 0.0,
            jumping: false,
            jump_timer: 0.0,
            game_over: false,
            lives: STARTING_LIVES,
            boosting: false,
            boost_timer: 0.0,
            boost_cooldown: 0.0,
            spawn_timer: 0.0,
            rng: Rng::new(0xDEAD_BEEF),
            rotation_accumulator: 0.0,
            screen_width: arcade_cart::DESIGN_WIDTH,
            screen_height: arcade_cart::DESIGN_HEIGHT,
        }
    }

    fn current_speed(&self) -> f32 {
        if self.boosting {
            STANDARD_SPEED * BOOST_MULTIPLIER
        } else if self.jumping {
            STANDARD_SPEED * 1.4
        } else {
            STANDARD_SPEED
        }
    }

    fn try_spawn_obstacles(&mut self) {
        if self.obstacles.len() >= MAX_OBSTACLES {
            return;
        }

        let spawn_y = self.world_y + self.screen_height + 100.0;

        if self.rng.range(1000) < DROP_RATE_SMALL_TREE {
            let x = 30.0 + self.rng.f32() * (self.screen_width - 60.0);
            let y_offset = self.rng.f32() * 50.0;
            self.obstacles.push(Obstacle {
                x,
                y: spawn_y + y_offset,
                kind: ObstacleType::SmallTree,
                hit: false,
            });
        }

        if self.rng.range(1000) < DROP_RATE_TALL_TREE {
            let x = 30.0 + self.rng.f32() * (self.screen_width - 60.0);
            let y_offset = self.rng.f32() * 50.0;
            self.obstacles.push(Obstacle {
                x,
                y: spawn_y + y_offset,
                kind: ObstacleType::TallTree,
                hit: false,
            });
        }

        if self.rng.range(1000) < DROP_RATE_ROCK {
            let x = 30.0 + self.rng.f32() * (self.screen_width - 60.0);
            let y_offset = self.rng.f32() * 50.0;
            self.obstacles.push(Obstacle {
                x,
                y: spawn_y + y_offset,
                kind: ObstacleType::Rock,
                hit: false,
            });
        }

        if self.rng.range(1000) < DROP_RATE_JUMP {
            let x = 50.0 + self.rng.f32() * (self.screen_width - 100.0);
            let y_offset = self.rng.f32() * 50.0;
            self.obstacles.push(Obstacle {
                x,
                y: spawn_y + y_offset,
                kind: ObstacleType::Jump,
                hit: false,
            });
        }
    }

    fn check_collisions(&mut self) {
        if self.crashed || self.jumping {
            return;
        }

        let skier_world_y = self.world_y + self.screen_height * 0.5;
        let skier_width = 20.0;
        let skier_height = 24.0;
        let skier_left = self.skier_x - skier_width / 2.0;
        let skier_right = self.skier_x + skier_width / 2.0;
        let skier_top = skier_world_y - skier_height / 2.0;
        let skier_bottom = skier_world_y + skier_height / 2.0;

        for obs in &mut self.obstacles {
            if obs.hit {
                continue;
            }

            let obs_width = obs.kind.collision_width();
            let obs_height = obs.kind.collision_height();
            let obs_left = obs.x - obs_width / 2.0;
            let obs_right = obs.x + obs_width / 2.0;
            let obs_top = obs.y - obs_height / 2.0;
            let obs_bottom = obs.y + obs_height / 2.0;

            if skier_right > obs_left
                && skier_left < obs_right
                && skier_bottom > obs_top
                && skier_top < obs_bottom
            {
                obs.hit = true;

                if obs.kind == ObstacleType::Jump {
                    self.jumping = true;
                    self.jump_timer = 1.0;
                } else {
                    self.crashed = true;
                    self.crash_timer = CRASH_RECOVERY_TIME;
                }
                break;
            }
        }
    }

    fn cleanup_obstacles(&mut self) {
        let min_y = self.world_y - 200.0;
        self.obstacles.retain(|obs| obs.y > min_y);
    }

    fn try_spawn_monster(&mut self) {
        if self.distance_metres < MONSTER_DISTANCE_THRESHOLD {
            return;
        }
        if self.monster.is_some() {
            return;
        }

        if self.rng.range(1000) < MONSTER_SPAWN_RATE {
            let spawn_x = 100.0 + self.rng.f32() * (self.screen_width - 200.0);
            let spawn_y = self.world_y - 200.0;
            self.monster = Some(Monster::new(spawn_x, spawn_y));
        }
    }

    fn update_monster(&mut self, dt: f32) {
        let skier_world_y = self.world_y + self.screen_height * 0.5;

        if let Some(monster) = &mut self.monster
            && monster.active
        {
            monster.update(dt, self.skier_x, skier_world_y);
            if monster.caught_skier(self.skier_x, skier_world_y) {
                monster.start_eating();
                self.being_eaten = true;
            }
        }
    }

    fn get_skier_sprite(&self) -> SpriteRegion {
        if self.crashed {
            SKIER_HIT
        } else if self.jumping {
            SKIER_JUMPING
        } else {
            self.direction.sprite_region()
        }
    }
}

impl Default for SkiFree {
    fn default() -> Self {
        Self::new()
    }
}

impl Game for SkiFree {
    fn update(&mut self, dt: f32, input: &Input) -> bool {
        if input.exit_requested || input.menu_requested {
            return true;
        }

        if self.game_over {
            if input.action_pressed {
                *self = Self::new();
            }
            return false;
        }

        if self.boost_cooldown > 0.0 {
            self.boost_cooldown -= dt;
        }

        if self.boosting {
            self.boost_timer -= dt;
            if self.boost_timer <= 0.0 {
                self.boosting = false;
                self.boost_cooldown = BOOST_COOLDOWN;
            }
        }

        if self.being_eaten {
            if let Some(monster) = &mut self.monster {
                let skier_world_y = self.world_y + self.screen_height * 0.5;
                if monster.update(dt, self.skier_x, skier_world_y) {
                    self.being_eaten = false;
                    self.lives = self.lives.saturating_sub(1);

                    if self.lives == 0 {
                        self.game_over = true;
                    } else {
                        self.monster = None;
                        self.direction = SkierDirection::South;
                    }
                }
            }
            return false;
        }

        if self.crashed {
            self.crash_timer -= dt;
            if self.crash_timer <= 0.0 && input.action_pressed {
                self.crashed = false;
                self.direction = SkierDirection::South;
            }
            self.update_monster(dt);
            return false;
        }

        if self.jumping {
            self.jump_timer -= dt;
            if self.jump_timer <= 0.0 {
                self.jumping = false;
            }
        }

        if input.action_pressed && !self.boosting && self.boost_cooldown <= 0.0 {
            self.boosting = true;
            self.boost_timer = BOOST_DURATION;
        }

        let capped_rotation = input.rotation.clamp(-TURN_THRESHOLD * 1.5, TURN_THRESHOLD * 1.5);
        self.rotation_accumulator += capped_rotation;

        if self.rotation_accumulator > TURN_THRESHOLD {
            self.rotation_accumulator -= TURN_THRESHOLD;
            self.direction = self.direction.turn_right();
        } else if self.rotation_accumulator < -TURN_THRESHOLD {
            self.rotation_accumulator += TURN_THRESHOLD;
            self.direction = self.direction.turn_left();
        }

        if !self.direction.is_stopped() {
            let (x_factor, y_factor) = self.direction.speed_factors();
            let speed = self.current_speed();

            self.skier_x += x_factor * speed * dt;
            self.skier_x = self.skier_x.clamp(30.0, self.screen_width - 30.0);

            let distance_moved = y_factor * speed * dt;
            self.world_y += distance_moved;
            self.distance_metres = self.world_y / PIXELS_PER_METRE;
        }

        self.spawn_timer -= dt;
        if self.spawn_timer <= 0.0 {
            self.try_spawn_obstacles();
            self.try_spawn_monster();
            self.spawn_timer = SPAWN_CHECK_INTERVAL;
        }

        self.update_monster(dt);
        self.check_collisions();
        self.cleanup_obstacles();

        false
    }

    fn render(&self, ui: &mut Ui) {
        let rect = ui.available_rect_before_wrap();
        let painter = ui.painter();

        // Game→screen scale. At native 1080×1920 these are 1.0.
        let sx = rect.width() / self.screen_width;
        let sy = rect.height() / self.screen_height;
        let uniform = sx.min(sy);
        let sprite_scale = 2.0 * uniform;

        painter.rect_filled(rect, 0.0, Color32::WHITE);

        let char_image = Image::new(egui::include_image!("../assets/sprite-characters.png"))
            .texture_options(TextureOptions::NEAREST);
        let obj_image = Image::new(egui::include_image!("../assets/skifree-objects.png"))
            .texture_options(TextureOptions::NEAREST);

        // Sort obstacles by Y for correct depth layering.
        let mut sorted_obstacles: Vec<&Obstacle> = self.obstacles.iter().collect();
        sorted_obstacles.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal));

        for obs in &sorted_obstacles {
            let screen_y_game = obs.y - self.world_y;
            if screen_y_game < -100.0 || screen_y_game > self.screen_height + 50.0 {
                continue;
            }

            let sprite = obs.kind.sprite_region();
            let uv = sprite.uv(OBJ_SHEET_WIDTH, OBJ_SHEET_HEIGHT);
            let size = Vec2::new(sprite.w * sprite_scale, sprite.h * sprite_scale);
            let screen_pos = Pos2::new(
                rect.min.x + obs.x * sx - size.x / 2.0,
                rect.min.y + screen_y_game * sy - size.y / 2.0,
            );
            let sprite_rect = Rect::from_min_size(screen_pos, size);
            obj_image.clone().uv(uv).paint_at(ui, sprite_rect);
        }

        if !self.being_eaten {
            let skier_sprite = self.get_skier_sprite();
            let skier_uv = skier_sprite.uv(CHAR_SHEET_WIDTH, CHAR_SHEET_HEIGHT);
            let skier_size = Vec2::new(skier_sprite.w * sprite_scale, skier_sprite.h * sprite_scale);
            let skier_screen_x = rect.min.x + self.skier_x * sx - skier_size.x / 2.0;
            let skier_screen_y = rect.min.y + self.screen_height * 0.5 * sy - skier_size.y / 2.0;
            let skier_rect = Rect::from_min_size(Pos2::new(skier_screen_x, skier_screen_y), skier_size);
            char_image.clone().uv(skier_uv).paint_at(ui, skier_rect);
        }

        if let Some(monster) = &self.monster
            && (monster.active || monster.eating)
        {
            let monster_screen_y_game = monster.y - self.world_y;
            if monster_screen_y_game > -100.0 && monster_screen_y_game < self.screen_height + 100.0 {
                let moving_east = monster.x < self.skier_x;
                let monster_sprite = monster.sprite(moving_east);
                let monster_uv = monster_sprite.uv(CHAR_SHEET_WIDTH, CHAR_SHEET_HEIGHT);
                let monster_size =
                    Vec2::new(monster_sprite.w * sprite_scale, monster_sprite.h * sprite_scale);
                let monster_screen_x = rect.min.x + monster.x * sx - monster_size.x / 2.0;
                let monster_screen_pos_y =
                    rect.min.y + monster_screen_y_game * sy - monster_size.y / 2.0;
                let monster_rect = Rect::from_min_size(
                    Pos2::new(monster_screen_x, monster_screen_pos_y),
                    monster_size,
                );
                char_image
                    .clone()
                    .uv(monster_uv)
                    .paint_at(ui, monster_rect);
            }
        }

        // === HUD ===

        let distance_text = format!("{}m", self.distance_metres as u32);
        painter.text(
            Pos2::new(rect.max.x - 20.0, rect.min.y + 30.0),
            egui::Align2::RIGHT_TOP,
            &distance_text,
            egui::FontId::monospace(32.0 * uniform),
            Color32::BLACK,
        );

        let lives_text = format!("♥ x{}", self.lives);
        painter.text(
            Pos2::new(rect.min.x + 20.0, rect.min.y + 30.0),
            egui::Align2::LEFT_TOP,
            &lives_text,
            egui::FontId::monospace(28.0 * uniform),
            Color32::from_rgb(213, 94, 0),
        );

        if self.boosting {
            let boost_bar_width = 120.0 * (self.boost_timer / BOOST_DURATION) * uniform;
            let bar_rect = Rect::from_min_size(
                Pos2::new(rect.min.x + 20.0, rect.min.y + 65.0 * uniform),
                Vec2::new(boost_bar_width, 12.0 * uniform),
            );
            painter.rect_filled(bar_rect, 2.0, Color32::from_rgb(0, 158, 115));

            painter.text(
                Pos2::new(rect.min.x + 150.0 * uniform, rect.min.y + 70.0 * uniform),
                egui::Align2::LEFT_CENTER,
                "BOOST!",
                egui::FontId::monospace(20.0 * uniform),
                Color32::from_rgb(0, 158, 115),
            );
        } else if self.boost_cooldown > 0.0 {
            let cooldown_text = format!("Boost: {:.0}s", self.boost_cooldown);
            painter.text(
                Pos2::new(rect.min.x + 20.0, rect.min.y + 70.0 * uniform),
                egui::Align2::LEFT_CENTER,
                &cooldown_text,
                egui::FontId::monospace(18.0 * uniform),
                Color32::GRAY,
            );
        } else if self.distance_metres >= MONSTER_DISTANCE_THRESHOLD {
            painter.text(
                Pos2::new(rect.min.x + 20.0, rect.min.y + 70.0 * uniform),
                egui::Align2::LEFT_CENTER,
                "Boost: READY ●",
                egui::FontId::monospace(18.0 * uniform),
                Color32::from_rgb(0, 158, 115),
            );
        }

        if self.direction.is_stopped() {
            painter.text(
                Pos2::new(rect.min.x + 20.0, rect.min.y + 100.0 * uniform),
                egui::Align2::LEFT_TOP,
                "STOPPED",
                egui::FontId::monospace(24.0 * uniform),
                Color32::from_rgb(213, 94, 0),
            );
        }

        if self.jumping {
            painter.text(
                Pos2::new(rect.center().x, rect.min.y + 80.0 * uniform),
                egui::Align2::CENTER_CENTER,
                "JUMPING!",
                egui::FontId::proportional(36.0 * uniform),
                Color32::from_rgb(0, 114, 178),
            );
        }

        if self.game_over {
            painter.text(
                Pos2::new(rect.center().x, rect.min.y + 200.0 * uniform),
                egui::Align2::CENTER_CENTER,
                "GAME OVER",
                egui::FontId::proportional(64.0 * uniform),
                Color32::from_rgb(213, 94, 0),
            );

            painter.text(
                Pos2::new(rect.center().x, rect.min.y + 280.0 * uniform),
                egui::Align2::CENTER_CENTER,
                "The yeti got you!",
                egui::FontId::proportional(32.0 * uniform),
                Color32::BLACK,
            );

            painter.text(
                Pos2::new(rect.center().x, rect.min.y + 340.0 * uniform),
                egui::Align2::CENTER_CENTER,
                format!("Final distance: {}m", self.distance_metres as u32),
                egui::FontId::proportional(36.0 * uniform),
                Color32::BLACK,
            );

            painter.text(
                Pos2::new(rect.center().x, rect.min.y + 400.0 * uniform),
                egui::Align2::CENTER_CENTER,
                "Press button to play again",
                egui::FontId::proportional(28.0 * uniform),
                Color32::DARK_GRAY,
            );
        } else if self.being_eaten {
            painter.text(
                Pos2::new(rect.center().x, rect.min.y + 200.0 * uniform),
                egui::Align2::CENTER_CENTER,
                "OH NO!",
                egui::FontId::proportional(64.0 * uniform),
                Color32::from_rgb(213, 94, 0),
            );

            let lives_after = self.lives.saturating_sub(1);
            if lives_after > 0 {
                painter.text(
                    Pos2::new(rect.center().x, rect.min.y + 280.0 * uniform),
                    egui::Align2::CENTER_CENTER,
                    format!("{lives_after} lives remaining"),
                    egui::FontId::proportional(28.0 * uniform),
                    Color32::DARK_GRAY,
                );
            }
        } else if self.crashed {
            painter.text(
                Pos2::new(rect.center().x, rect.min.y + 200.0 * uniform),
                egui::Align2::CENTER_CENTER,
                "OUCH!",
                egui::FontId::proportional(64.0 * uniform),
                Color32::from_rgb(213, 94, 0),
            );

            if self.crash_timer <= 0.0 {
                painter.text(
                    Pos2::new(rect.center().x, rect.min.y + 280.0 * uniform),
                    egui::Align2::CENTER_CENTER,
                    "Press button to continue",
                    egui::FontId::proportional(28.0 * uniform),
                    Color32::DARK_GRAY,
                );
            }
        }

        if self.distance_metres > 1800.0 && self.distance_metres < 2000.0 && self.monster.is_none() {
            painter.text(
                Pos2::new(rect.center().x, rect.min.y + 120.0 * uniform),
                egui::Align2::CENTER_CENTER,
                "⚠ YETI TERRITORY AHEAD ⚠",
                egui::FontId::proportional(28.0 * uniform),
                Color32::from_rgb(213, 94, 0),
            );
        }

        painter.text(
            Pos2::new(rect.center().x, rect.max.y - 50.0 * uniform),
            egui::Align2::CENTER_CENTER,
            "STEER: ROTATE  •  BOOST: PRESS  •  MENU: ESC",
            egui::FontId::proportional(22.0 * uniform),
            Color32::GRAY,
        );
    }

    fn name(&self) -> &str {
        "SKIFREE"
    }

    fn description(&self) -> &str {
        "WATCH OUT FOR TREES"
    }

    fn year(&self) -> &str {
        "1991"
    }

    fn debug_stats(&self) -> Option<String> {
        let monster_status = if self.monster.is_some() { "active" } else { "none" };
        Some(format!(
            "obstacles={}/{}, dist={}m, dir={:?}, crashed={}, jumping={}, monster={}",
            self.obstacles.len(),
            MAX_OBSTACLES,
            self.distance_metres as u32,
            self.direction,
            self.crashed,
            self.jumping,
            monster_status
        ))
    }
}
