use eframe::{egui, App, CreationContext};
use egui::{Color32, Pos2, Vec2};
use horus::prelude::*;
use std::time::{Duration, Instant};

const GRID_WIDTH: usize = 20;
const GRID_HEIGHT: usize = 20;

const BG_COLOR: Color32 = Color32::from_rgb(30, 30, 30);
const GRID_LINE_COLOR: Color32 = Color32::from_rgb(50, 50, 50);
const SNAKE_BODY_COLOR: Color32 = Color32::from_rgb(0, 200, 100);
const SNAKE_HEAD_COLOR: Color32 = Color32::from_rgb(0, 255, 150);
const EYE_COLOR: Color32 = Color32::WHITE;

pub fn main() -> std::result::Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 500.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Snakesim GUI",
        options,
        Box::new(|_cc: &CreationContext| Box::new(SnakesimNode::new())),
    )
}

struct SnakesimNode {
    snake: Vec<(usize, usize)>,
    last_update: Instant,
    direction: (isize, isize),
    snake_state_subscriber: Hub<SnakeState>,
}
impl SnakesimNode {
    pub fn new() -> Self {
        Self {
            snake: vec![(5, 5), (4, 5), (3, 5)],
            last_update: Instant::now(),
            direction: (1, 0),
            snake_state_subscriber: Hub::new("snakestate")
                .expect("failed to create snake state subscriber"),
        }
    }
    pub fn control(&mut self) {
        // Consume ALL available messages, keep only the latest direction
        while let Some(snake_state) = self.snake_state_subscriber.recv(None) {
            match snake_state.direction {
                1 => self.direction = (0, -1), // Up
                2 => self.direction = (0, 1),  // Down
                3 => self.direction = (-1, 0), // Left
                4 => self.direction = (1, 0),  // Right
                _ => {}                        // Ignore unknown directions
            }
        }
    }
}
impl Node for SnakesimNode {
    fn name(&self) -> &'static str {
        "keyboardInputNode"
    }
    fn tick(&mut self, _: Option<&mut NodeInfo>) {
        self.control();
        let (dx, dy) = self.direction;
        let (head_x, head_y) = self.snake[0];
        let new_head = (
            ((head_x as isize + dx + GRID_WIDTH as isize) % GRID_WIDTH as isize) as usize,
            ((head_y as isize + dy + GRID_HEIGHT as isize) % GRID_HEIGHT as isize) as usize,
        );
        self.snake.insert(0, new_head);
        self.snake.pop();
    }
}

impl App for SnakesimNode {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Move every 200ms
        if self.last_update.elapsed() > Duration::from_millis(200) {
            self.last_update = Instant::now();
            self.tick(None);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            let cell_width = rect.width() / GRID_WIDTH as f32;
            let cell_height = rect.height() / GRID_HEIGHT as f32;

            let painter = ui.painter();
            painter.rect_filled(rect, 0.0, BG_COLOR);

            // Grid lines
            for i in 0..=GRID_WIDTH {
                let x = rect.left() + i as f32 * cell_width;
                painter.line_segment(
                    [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                    (1.0, GRID_LINE_COLOR),
                );
            }

            for j in 0..=GRID_HEIGHT {
                let y = rect.top() + j as f32 * cell_height;
                painter.line_segment(
                    [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                    (1.0, GRID_LINE_COLOR),
                );
            }

            // Snake segments
            for (i, &(x, y)) in self.snake.iter().enumerate() {
                let pos = Pos2::new(
                    rect.left() + x as f32 * cell_width,
                    rect.top() + y as f32 * cell_height,
                );
                let square = egui::Rect::from_min_size(pos, Vec2::new(cell_width, cell_height));
                let color = if i == 0 {
                    SNAKE_HEAD_COLOR
                } else {
                    SNAKE_BODY_COLOR
                };
                painter.rect_filled(square, 6.0, color);

                // Eyes for head
                if i == 0 {
                    let eye_radius = (cell_width.min(cell_height) * 0.1).max(2.0);
                    let center = square.center();
                    let (dx, dy) = self.direction;
                    let offset = Vec2::new(dx as f32, dy as f32) * (cell_width * 0.2);

                    let eye_offset = cell_width * 0.15;
                    let eye1 = center + offset + Vec2::new(-eye_offset, -eye_offset);
                    let eye2 = center + offset + Vec2::new(eye_offset, -eye_offset);

                    painter.circle_filled(eye1, eye_radius, EYE_COLOR);
                    painter.circle_filled(eye2, eye_radius, EYE_COLOR);
                }
            }
        });

        ctx.request_repaint(); // Continually redraw
    }
}
