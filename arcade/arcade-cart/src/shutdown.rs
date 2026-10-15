//! CRT power-off animation overlay.
//!
//! Four-phase classic-CRT effect: vertical squeeze → bright line hold →
//! horizontal squeeze to a dot → phosphor fade. Painted as an overlay on top
//! of whatever the current scene is.

use egui::{Color32, Pos2, Rect, Ui, Vec2};

const VERTICAL_DURATION: f32 = 0.45;
const LINE_HOLD: f32 = 0.25;
const HORIZONTAL_DURATION: f32 = 0.5;
const FADE_DURATION: f32 = 0.5;

/// Total time from `start()` to "complete".
pub const TOTAL_DURATION: f32 = VERTICAL_DURATION + LINE_HOLD + HORIZONTAL_DURATION + FADE_DURATION;

/// CRT shutdown animation state.
#[derive(Default)]
pub struct Shutdown {
    timer: f32,
    active: bool,
}

impl Shutdown {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin (or restart) the animation from t=0.
    pub fn start(&mut self) {
        self.active = true;
        self.timer = 0.0;
    }

    /// Is the animation currently playing?
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Advance the timer by `dt`. Returns `true` once the animation has
    /// played to completion (host should perform whatever the shutdown
    /// represents, e.g. exit the console).
    pub fn update(&mut self, dt: f32) -> bool {
        if !self.active {
            return false;
        }
        self.timer += dt;
        if self.timer >= TOTAL_DURATION {
            self.active = false;
            return true;
        }
        false
    }

    /// Paint the current frame of the animation as an overlay covering `ui`'s
    /// available rect. No-op when `is_active()` is false.
    pub fn render(&self, ui: &mut Ui) {
        if !self.active {
            return;
        }

        let rect = ui.available_rect_before_wrap();
        let painter = ui.painter();
        let center = rect.center();
        let t = self.timer;

        let (phase, phase_progress) = if t < VERTICAL_DURATION {
            (1, t / VERTICAL_DURATION)
        } else if t < VERTICAL_DURATION + LINE_HOLD {
            (2, (t - VERTICAL_DURATION) / LINE_HOLD)
        } else if t < VERTICAL_DURATION + LINE_HOLD + HORIZONTAL_DURATION {
            (3, (t - VERTICAL_DURATION - LINE_HOLD) / HORIZONTAL_DURATION)
        } else {
            (
                4,
                (t - VERTICAL_DURATION - LINE_HOLD - HORIZONTAL_DURATION) / FADE_DURATION,
            )
        };

        let ease_out_cubic = |p: f32| 1.0 - (1.0 - p).powi(3);
        let ease_out_expo = |p: f32| {
            if p >= 1.0 {
                1.0
            } else {
                1.0 - 2.0_f32.powf(-10.0 * p)
            }
        };

        // Phase 1 & 2: black bars compressing from top and bottom
        let vertical_progress = if phase == 1 {
            ease_out_cubic(phase_progress)
        } else {
            1.0
        };

        let squeeze_height = rect.height() * (1.0 - vertical_progress);
        let half_squeeze = squeeze_height / 2.0;

        // Scanline interference during phase 1
        if phase == 1 && vertical_progress < 0.95 {
            let flicker_intensity = ((t * 60.0).sin() * 0.5 + 0.5) * 30.0;
            for y in (0..rect.height() as i32).step_by(4) {
                let yf = rect.min.y + y as f32;
                if yf > center.y - half_squeeze && yf < center.y + half_squeeze {
                    let line_flicker = ((y as f32 * 0.1 + t * 30.0).sin() * flicker_intensity) as u8;
                    painter.rect_filled(
                        Rect::from_min_size(
                            Pos2::new(rect.min.x, yf),
                            Vec2::new(rect.width(), 1.0),
                        ),
                        0.0,
                        Color32::from_rgba_unmultiplied(0, 0, 0, line_flicker.min(40)),
                    );
                }
            }
        }

        // Top black bar with phosphor edge
        let top_bar_bottom = center.y - half_squeeze;
        painter.rect_filled(
            Rect::from_min_max(rect.min, Pos2::new(rect.max.x, top_bar_bottom)),
            0.0,
            Color32::BLACK,
        );
        if phase == 1 && half_squeeze > 10.0 {
            for i in 0..6 {
                let edge_y = top_bar_bottom + i as f32;
                let alpha = ((6 - i) as f32 / 6.0 * 40.0) as u8;
                painter.rect_filled(
                    Rect::from_min_size(
                        Pos2::new(rect.min.x, edge_y),
                        Vec2::new(rect.width(), 1.0),
                    ),
                    0.0,
                    Color32::from_rgba_unmultiplied(100, 200, 255, alpha),
                );
            }
        }

        // Bottom black bar with phosphor edge
        let bottom_bar_top = center.y + half_squeeze;
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(rect.min.x, bottom_bar_top), rect.max),
            0.0,
            Color32::BLACK,
        );
        if phase == 1 && half_squeeze > 10.0 {
            for i in 0..6 {
                let edge_y = bottom_bar_top - i as f32 - 1.0;
                let alpha = ((6 - i) as f32 / 6.0 * 40.0) as u8;
                painter.rect_filled(
                    Rect::from_min_size(
                        Pos2::new(rect.min.x, edge_y),
                        Vec2::new(rect.width(), 1.0),
                    ),
                    0.0,
                    Color32::from_rgba_unmultiplied(100, 200, 255, alpha),
                );
            }
        }

        // Phase 2, 3, 4: glowing horizontal line
        if phase >= 2 {
            let line_width = if phase == 2 {
                let breathe = (phase_progress * std::f32::consts::PI * 2.0).sin() * 0.01 + 1.0;
                rect.width() * breathe
            } else if phase == 3 {
                rect.width() * (1.0 - ease_out_expo(phase_progress))
            } else {
                let dot_size = rect.width() * 0.015 * (1.0 - phase_progress * 0.5);
                dot_size.max(3.0)
            };

            let line_height = if phase == 2 {
                6.0
            } else if phase == 3 {
                6.0 - 2.0 * phase_progress
            } else {
                (4.0 * (1.0 - phase_progress * 0.8)).max(2.0)
            };

            let brightness = if phase == 4 {
                ((1.0 - ease_out_cubic(phase_progress)) * 255.0) as u8
            } else {
                255
            };

            if line_width > 2.0 {
                // Outermost glow
                painter.rect_filled(
                    Rect::from_center_size(center, Vec2::new(line_width + 40.0, line_height + 30.0)),
                    f32::midpoint(line_height, 30.0),
                    Color32::from_rgba_unmultiplied(0, 180, 220, brightness / 12),
                );
                painter.rect_filled(
                    Rect::from_center_size(center, Vec2::new(line_width + 24.0, line_height + 20.0)),
                    f32::midpoint(line_height, 20.0),
                    Color32::from_rgba_unmultiplied(0, 200, 240, brightness / 8),
                );
                painter.rect_filled(
                    Rect::from_center_size(center, Vec2::new(line_width + 14.0, line_height + 12.0)),
                    f32::midpoint(line_height, 12.0),
                    Color32::from_rgba_unmultiplied(50, 220, 255, brightness / 4),
                );
                painter.rect_filled(
                    Rect::from_center_size(center, Vec2::new(line_width + 6.0, line_height + 5.0)),
                    f32::midpoint(line_height, 5.0),
                    Color32::from_rgba_unmultiplied(150, 240, 255, brightness / 2),
                );
            }

            // Core line
            painter.rect_filled(
                Rect::from_center_size(center, Vec2::new(line_width.max(2.0), line_height.max(2.0))),
                line_height.max(2.0) / 2.0,
                Color32::from_rgba_unmultiplied(255, 255, 255, brightness),
            );

            // Scanline texture during phase 2
            if phase == 2 {
                for x in (0..(line_width as i32)).step_by(3) {
                    let xf = center.x - line_width / 2.0 + x as f32;
                    let scan_alpha = ((x as f32 * 0.3 + t * 100.0).sin() * 15.0 + 15.0) as u8;
                    painter.rect_filled(
                        Rect::from_center_size(
                            Pos2::new(xf, center.y),
                            Vec2::new(1.0, line_height),
                        ),
                        0.0,
                        Color32::from_rgba_unmultiplied(0, 0, 0, scan_alpha),
                    );
                }
            }
        }

        // Phase 4: phosphor afterimage
        if phase == 4 {
            let ghost_alpha = ((1.0 - phase_progress) * 20.0) as u8;
            if ghost_alpha > 0 {
                painter.rect_filled(
                    Rect::from_center_size(center, Vec2::new(rect.width() * 0.3, 2.0)),
                    1.0,
                    Color32::from_rgba_unmultiplied(0, 150, 180, ghost_alpha),
                );
            }
        }

        // Phase 4 tail: smooth fade to full black
        if phase == 4 && phase_progress >= 0.6 {
            let black_alpha = (((phase_progress - 0.6) / 0.4).powf(0.5) * 255.0) as u8;
            painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(0, 0, 0, black_alpha));
        }
    }
}
