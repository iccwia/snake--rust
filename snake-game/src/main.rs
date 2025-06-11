extern crate glutin_window;
extern crate graphics;
extern crate opengl_graphics;
extern crate piston;
use std::collections::HashSet;

use glutin_window::GlutinWindow as Window;
use opengl_graphics::{GlGraphics, OpenGL};
use piston::event_loop::{EventLoop, EventSettings, Events};
use piston::input::{RenderArgs, RenderEvent, UpdateArgs, UpdateEvent};
use piston::window::WindowSettings;
use piston::{Button, ButtonArgs, ButtonEvent, ButtonState, Key};
use rand::distributions::Standard;
use rand::prelude::Distribution;
use rand::seq::SliceRandom;
use rand::thread_rng;

const SQUARE_SIZE: i16 = 9;
const WINDOW_WIDTH_SIZE: i16 = 300;
const WINDOW_HEIGHT_SIZE: i16 = 300;

// 生成随机位置
fn generate_random_position(width: i16, height: i16) -> Position {
    let mut rng = thread_rng();
    let width_range: Vec<i16> = (0..=(width - SQUARE_SIZE))
        .step_by(SQUARE_SIZE.try_into().unwrap())
        .collect();
    let height_range: Vec<i16> = (0..=(height - SQUARE_SIZE))
        .step_by(SQUARE_SIZE.try_into().unwrap())
        .collect();
    Position {
        x: *width_range.choose(&mut rng).unwrap_or(&0),
        y: *height_range.choose(&mut rng).unwrap_or(&0),
    }
}

// 生成画布内所有格子
fn generate_all_position(width: i16, height: i16) -> HashSet<Position> {
    let mut all_positions_vec: Vec<Position> = (0..=(width - SQUARE_SIZE))
        .step_by(SQUARE_SIZE.try_into().unwrap())
        .flat_map(|x| {
            (0..=(height - SQUARE_SIZE))
                .step_by(SQUARE_SIZE.try_into().unwrap())
                .map(move |y| Position { x, y })
        })
        .collect();
    all_positions_vec.shuffle(&mut thread_rng());
    all_positions_vec.into_iter().collect()
}

// 蛇身方向检测 - 不可 180° 掉头
fn is_direction_conflict(prev_direction: &Direction, new_direction: Direction) -> bool {
    match (prev_direction, new_direction) {
        (Direction::Up, Direction::Down) |
        (Direction::Down, Direction::Up) |
        (Direction::Left, Direction::Right) |
        (Direction::Right, Direction::Left) => true,
        _ => false,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct Position {
    x: i16,
    y: i16,
}

struct Food {
    position: Position,
}

impl Food {
    fn new(width: i16, height: i16) -> Self {
        let init_position = generate_random_position(width, height);

        Food {
            position: init_position,
        }
    }

    fn refresh_position(
        &self,
        all_positions: &HashSet<Position>,
        invalid_positions: &HashSet<Position>,
    ) -> Option<Position> {
        let available_positions: Vec<&Position> = all_positions
            .difference(invalid_positions)
            .collect();

        if available_positions.is_empty() {
            None
        } else {
            let mut rng = thread_rng();
            // 修复：解引用两次获取 Position 类型
            Some(**available_positions.choose(&mut rng).unwrap())
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Distribution<Direction> for Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Direction {
        match rng.gen_range(0..=3) {
            0 => Direction::Up,
            1 => Direction::Down,
            2 => Direction::Left,
            _ => Direction::Right,
        }
    }
}

pub struct Snake {
    body: Vec<Position>,
    body_set: HashSet<Position>,
    direction: Direction,
}

impl Snake {
    fn new(width: i16, height: i16) -> Self {
        let init_direction: Direction = rand::random();
        let init_position = generate_random_position(width, height);
        let mut body_vec = Vec::new();
        let mut body_hashset = HashSet::new();
        body_vec.push(init_position);
        body_hashset.insert(init_position);
        Snake {
            body: body_vec,
            body_set: body_hashset,
            direction: init_direction,
        }
    }

    // 获取蛇头位置
    fn head(&self) -> &Position {
        &self.body[0]
    }

    // 移动蛇
    fn move_forward(&mut self, grow: bool) {
        let head = self.head();
        let new_head = match self.direction {
            Direction::Up => Position { x: head.x, y: head.y - SQUARE_SIZE },
            Direction::Down => Position { x: head.x, y: head.y + SQUARE_SIZE },
            Direction::Left => Position { x: head.x - SQUARE_SIZE, y: head.y },
            Direction::Right => Position { x: head.x + SQUARE_SIZE, y: head.y },
        };

        self.body.insert(0, new_head);
        self.body_set.insert(new_head);

        if !grow {
            if let Some(tail) = self.body.pop() {
                self.body_set.remove(&tail);
            }
        }
    }
}

pub struct App {
    gl: GlGraphics,
    snake: Snake,
    food: Food,
    game_over: bool,
    all_position: HashSet<Position>,
    score: u32,
}

impl App {
    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
        const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
        const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
        const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];

        let snake_segments: Vec<[f64; 4]> = self.snake.body.iter()
            .map(|pos| rectangle::square(pos.x as f64, pos.y as f64, SQUARE_SIZE.into()))
            .collect();

        let food = rectangle::square(
            self.food.position.x as f64,
            self.food.position.y as f64,
            SQUARE_SIZE.into(),
        );

        self.gl.draw(args.viewport(), |c, gl| {
            clear(WHITE, gl);
            let transform = c.transform;

            // 绘制蛇身
            for (i, segment) in snake_segments.iter().enumerate() {
                let color = if i == 0 { RED } else { BLUE };
                rectangle(color, *segment, transform, gl);
            }

            // 绘制食物
            rectangle(GREEN, food, transform, gl);
        })
    }

    fn update(&mut self, _args: &UpdateArgs) {
        if self.game_over {
            return;
        }

        // 检查是否吃到食物
        let grow = self.snake.head().x == self.food.position.x
            && self.snake.head().y == self.food.position.y;

        // 移动蛇
        self.snake.move_forward(grow);

        // 检查碰撞
        if self.is_collision() {
            self.game_over = true;
            return;
        }

        // 如果吃到食物，生成新食物
        if grow {
            self.score += 1;
            match self.food.refresh_position(&self.all_position, &self.snake.body_set) {
                Some(position) => self.food.position = position,
                None => {
                    // 没有可用位置，游戏胜利
                    self.game_over = true;
                    println!("恭喜你赢得了游戏！得分: {}", self.score);
                }
            };
        }
    }

    fn change_direction(&mut self, args: &ButtonArgs) {
        if args.state != ButtonState::Press {
            return;
        }

        let new_direction = match args.button {
            Button::Keyboard(Key::Up) => Direction::Up,
            Button::Keyboard(Key::Down) => Direction::Down,
            Button::Keyboard(Key::Left) => Direction::Left,
            Button::Keyboard(Key::Right) => Direction::Right,
            _ => return,
        };

        if !is_direction_conflict(&self.snake.direction, new_direction) {
            self.snake.direction = new_direction;
        }
    }

    fn is_collision(&self) -> bool {
        let head = self.snake.head();

        // 检查是否撞墙
        if head.x < 0 || head.x >= WINDOW_WIDTH_SIZE ||
           head.y < 0 || head.y >= WINDOW_HEIGHT_SIZE {
            return true;
        }

        // 检查是否撞到自己的身体 (正确排除蛇头)
        if self.snake.body.len() > 1 {
            // 创建一个不包含蛇头的body_set副本
            let mut body_without_head = self.snake.body_set.clone();
            body_without_head.remove(head);
            
            // 检查蛇头是否与身体其他部分碰撞
            if body_without_head.contains(head) {
                return true;
            }
        }

        false
    }
}

fn main() {
    let opengl = OpenGL::V3_2;

    let mut window: Window = WindowSettings::new(
        "贪吃蛇游戏",
        [WINDOW_WIDTH_SIZE as u32, WINDOW_HEIGHT_SIZE as u32],
    )
    .graphics_api(opengl)
    .resizable(false)
    .exit_on_esc(true)
    .build()
    .unwrap();

    let snake = Snake::new(WINDOW_WIDTH_SIZE, WINDOW_HEIGHT_SIZE);
    let food = Food::new(WINDOW_WIDTH_SIZE, WINDOW_HEIGHT_SIZE);
    let all_position = generate_all_position(WINDOW_WIDTH_SIZE, WINDOW_HEIGHT_SIZE);

    let mut app = App {
        gl: GlGraphics::new(opengl),
        snake,
        food,
        game_over: false,
        all_position,
        score: 0,
    };

    let mut events = Events::new(EventSettings::new()).ups(10);

    while let Some(e) = events.next(&mut window) {
        if let Some(args) = e.render_args() {
            app.render(&args);
        }

        if let Some(args) = e.update_args() {
            app.update(&args);
        }

        if app.game_over {
            println!("游戏结束！得分: {}", app.score);
            return;
        }

        if let Some(args) = e.button_args() {
            app.change_direction(&args);
        }
    }
}
