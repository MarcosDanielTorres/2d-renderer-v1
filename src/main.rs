#![allow(unused, dead_code)]
use std::f32::consts::FRAC_2_PI;

use winit::event::{Event, VirtualKeyCode, WindowEvent};

use bm::async_runner;

type Label<'a> = Option<&'a str>;

#[derive(Default)]
pub struct Enemy<'a> {
    label: Label<'a>,
    x: f32,
    y: f32,
    color: [f32; 4],
}

impl<'a> Enemy<'a> {
    pub fn new(x: f32, y: f32, color: [f32; 4], label: Label<'a>) -> Self {
        Self { x, y, color, label }
    }

    pub fn on_update(&mut self, engine: &mut bm::Engine, new_pos: (f32, f32)) {
        // self.x = new_pos.0;
        // self.y = new_pos.1;
        // println!("new pos of {:?}: ({}, {})", self.label, self.x, self.y);
    }

    pub fn on_render(&mut self, engine: &mut bm::Engine) {
        // enemy
        let color: [f32; 4] = [1.0, 0.0, 1.0, 1.0];
        let position = glam::vec3(self.x, self.y, 0.0);
        let scale = glam::Vec3::new(75.0, 75.0, 1.0);
        let rotation: f32 = 0.0;

        engine.prepare_quad_data(
            glam::Mat4::from_translation(position),
            glam::Mat4::from_scale(scale),
            glam::Mat4::from_rotation_z(rotation),
            self.color,
        );
    }
}

#[derive(Default)]
struct EnemyContainer<'a> {
    enemies: Vec<Enemy<'a>>,
}

impl<'a> EnemyContainer<'a> {
    pub fn on_update(&mut self, engine: &mut bm::Engine) {
        for enemy in self.enemies.iter_mut() {
            enemy.on_update(engine, (1.0, 1.0));
        }
    }

    pub fn on_render(&mut self, engine: &mut bm::Engine) {
        for enemy in self.enemies.iter_mut() {
            enemy.on_render(engine);
        }
    }

    pub fn add_enemy(&mut self, elem: Enemy<'a>) {
        self.enemies.push(elem)
    }
}

#[derive(Default, Debug)]
struct Player {
    x: f32,
    y: f32,
    speed: f32,
}

impl Player {
    pub fn update(&self, engine: &mut bm::Engine) {}

    pub fn on_event(&mut self, event: bm::MyEvent) {
        dbg!("player pos: ({}, {})", self.x, self.y);
        match event {
            bm::MyEvent::KeyboardInput {
                state: winit::event::ElementState::Pressed,
                virtual_keycode: VirtualKeyCode::W,
            } => {
                self.y += 0.003;
            }
            bm::MyEvent::KeyboardInput {
                state: winit::event::ElementState::Pressed,
                virtual_keycode: VirtualKeyCode::S,
            } => {
                self.y -= 0.003;
            }
            bm::MyEvent::KeyboardInput {
                state: winit::event::ElementState::Pressed,
                virtual_keycode: VirtualKeyCode::A,
            } => {
                self.x -= 0.003;
            }
            bm::MyEvent::KeyboardInput {
                state: winit::event::ElementState::Pressed,
                virtual_keycode: VirtualKeyCode::D,
            } => {
                self.x += 0.003;
            }
            _ => (),
        }
    }

    pub fn on_render(&self, engine: &mut bm::Engine) {
        // player
        let color: [f32; 4] = [0.0, 1.0, 1.0, 0.1];
        let position = glam::Vec3::new(self.x, self.y, 0.0) * self.speed;
        let scale = glam::Vec3::new(75.0, 75.0, 1.0);
        let rotation: f32 = 0.0;

        engine.prepare_quad_data(
            glam::Mat4::from_translation(position),
            glam::Mat4::from_scale(scale),
            glam::Mat4::from_rotation_z(rotation),
            color,
        )
    }
}

#[derive(Default)]
struct App<'a> {
    player: Player,
    container: EnemyContainer<'a>,
}

impl<'a> App<'a> {
    pub fn new() -> Self {
        let mut container = EnemyContainer::default();
        App::create_enemies(&mut container);

        let player = Player {
            x: 0.0,
            y: 0.0,
            speed: 25.0,
        };
        Self { container, player }
    }
    fn create_enemies(container: &mut EnemyContainer) {
        let color1: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
        let color2: [f32; 4] = [1.0, 0.0, 0.0, 0.3];
        let enemy1 = Enemy::new(0.5, -1.0, color1, Some("Cube 1"));
        let enemy2 = Enemy::new(-0.5, 1.0, color2, Some("Cube 2"));
        container.add_enemy(enemy1);
        container.add_enemy(enemy2);
    }
}

impl<'a> bm::Application for App<'a> {
    fn on_update(&mut self, engine: &mut bm::Engine) {
        self.player.update(engine);
        self.container.on_update(engine);
    }

    fn on_render(&mut self, engine: &mut bm::Engine) {
        self.player.on_render(engine);
        self.container.on_render(engine);
    }

    fn on_event(&mut self, engine: &mut bm::Engine, event: bm::MyEvent) {
        self.player.on_event(event);
    }
}

pub fn main() {
    let mut app = App::new();
    pollster::block_on(async_runner(app));
}
