#![allow(unused, dead_code)]
use std::f32::consts::{FRAC_2_PI, FRAC_PI_2, FRAC_PI_4, FRAC_PI_6, FRAC_PI_8};

use bm::async_runner;
use glam::*;
use winit::{
    event::{ElementState, Event, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

type Label<'a> = Option<&'a str>;

#[derive(Default)]
pub struct Enemy<'a> {
    label: Label<'a>,
    x: f32,
    y: f32,
    scale_x: f32,
    scale_y: f32,
    color: [f32; 4],
    health: f32,
    texture_id: String,
}

impl<'a> Enemy<'a> {
    pub fn new(x: f32, y: f32, color: [f32; 4], label: Label<'a>, texture_id: String) -> Self {
        Self {
            x,
            y,
            scale_x: 150.0,
            scale_y: 150.0,
            color,
            label,
            health: 100.0,
            texture_id,
        }
    }

    pub fn on_update(&mut self, engine: &mut bm::Engine, new_pos: (f32, f32), delta_time: f32) {
        if let Some(val) = self.label {
            if val == "Enemy 1" {
                // println!("Enemy 1: {:?}, {:?}", (self.x, self.y), self.health);
            }
        }
        // self.x = new_pos.0;
        // self.y = new_pos.1;
        // println!("new pos of {:?}: ({}, {})", self.label, self.x, self.y);
    }

    pub fn on_render(&mut self, engine: &mut bm::Engine) {
        // enemy
        let color: [f32; 4] = [1.0, 0.0, 1.0, 1.0];
        let position = vec3(self.x, self.y, 0.0);
        let scale = Vec3::new(self.scale_x, self.scale_y, 1.0);
        let angle: f32 = 0.0;

        // engine.render_quad(position, scale, angle, self.color, Some(include_bytes!("pikachu.png")));
        engine.render_quad(
            position,
            scale,
            angle,
            self.color,
            Some(self.texture_id.clone()),
        );
    }
}

#[derive(Default)]
struct EnemyContainer<'a> {
    enemies: Vec<Enemy<'a>>,
}

impl<'a> EnemyContainer<'a> {
    pub fn on_update(&mut self, engine: &mut bm::Engine, player: &Player, delta_time: f32) {
        for enemy in self.enemies.iter_mut() {
            if enemy.health < 0.0 {
                continue;
            }
            enemy.on_update(engine, (1.0, 1.0), delta_time);

            // check collisions
            let half_player_w = player.scale_x / 2.0;
            let half_player_h = player.scale_y / 2.0;
            let player_c_x = player.x;
            let player_c_y = player.x;

            let half_enemy_w = enemy.scale_x / 2.0;
            let half_enemy_h = enemy.scale_y / 2.0;
            let enemy_c_x = enemy.x;
            let enemy_c_y = enemy.x;

            let left_player = player_c_x - half_player_w;
            let right_player = player_c_x + half_player_w;
            let top_player = player_c_y + half_player_h;
            let bottom_player = player_c_y - half_player_h;

            let left_enemy = enemy_c_x - half_enemy_w;
            let right_enemy = enemy_c_x + half_enemy_w;
            let top_enemy = enemy_c_y + half_enemy_h;
            let bottom_enemy = enemy_c_y - half_enemy_h;

            if (left_player <= right_enemy && top_player >= bottom_enemy) {
                // enemy.health -= 1.0;
            }
        }
    }

    pub fn on_render(&mut self, engine: &mut bm::Engine) {
        for enemy in self.enemies.iter_mut() {
            if enemy.health >= 0.0 {
                enemy.on_render(engine);
            }
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

    amount_down: f32,
    amount_up: f32,
    amount_left: f32,
    amount_right: f32,

    scale_x: f32,
    scale_y: f32,
    speed: f32,
}

impl Player {
    pub fn update(&mut self, engine: &mut bm::Engine, delta_time: f32) {
        self.x += (self.amount_right + self.amount_left) * self.speed;
        self.y += (self.amount_up + self.amount_down) * self.speed;
    }

    pub fn on_event(&mut self, event: bm::MyEvent) {
        let amount = match event {
            bm::MyEvent::KeyboardInput {
                state: ElementState::Pressed,
                ..
            } => 1.0,
            bm::MyEvent::KeyboardInput {
                state: ElementState::Released,
                ..
            } => 0.0,
        };

        match event {
            bm::MyEvent::KeyboardInput {
                physical_key: PhysicalKey::Code(KeyCode::KeyW),
                ..
            } => {
                // self.y += 0.003 * self.speed;
                self.amount_up = amount;
            }

            bm::MyEvent::KeyboardInput {
                physical_key: PhysicalKey::Code(KeyCode::KeyS),
                ..
            } => {
                // self.y -= 0.003 * self.speed;
                self.amount_down = -amount;
            }

            bm::MyEvent::KeyboardInput {
                physical_key: PhysicalKey::Code(KeyCode::KeyA),
                ..
            } => {
                // self.x -= 0.003 * self.speed;
                self.amount_left = -amount;
            }

            bm::MyEvent::KeyboardInput {
                physical_key: PhysicalKey::Code(KeyCode::KeyD),
                ..
            } => {
                // self.x += 0.003 * self.speed;
                self.amount_right = amount;
            }
            _ => (),
        }
    }

    pub fn on_render(&self, engine: &mut bm::Engine) {
        // player
        let color: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
        let position = Vec3::new(self.x, self.y, 0.0);
        // this is in pixels, which is good
        let scale = Vec3::new(self.scale_x, self.scale_y, 1.0);
        let line_scale = Vec3::new(10.0, 0.0, 1.0);
        let angle: f32 = FRAC_2_PI;
        //let angle: f32 = FRAC_PI_2;
        //let angle: f32 = 0.0;

        engine.render_quad(position, scale, angle, color, None);

        let mut orig = glam::Vec3::new(self.x - self.scale_x / 2.0, self.y, 0.0);
        let mut dest = glam::Vec3::new(self.x + self.scale_x / 2.0, self.y, 0.0);
        //let mut orig = Vec3::new(800.0 / 2.0, 600.0 / 2.0, 0.0);
        //let mut dest = Vec3::new(200.0, 150.0, 0.0);
        let line_color: [f32; 4] = [1.0, 0.3, 0.7, 1.0];

        engine.render_line(orig, dest, line_color);
        /////////////////////////////////
        // for rotations. It's useful////
        /////////////////////////////////
        let s = angle.sin();
        let c = angle.cos();

        orig.x -= self.x;
        orig.y -= self.y;

        let xnew = orig.x * c - orig.y * s;
        let ynew = orig.x * s + orig.y * c;

        orig.x = xnew + self.x;
        orig.y = ynew + self.y;

        dest.x -= self.x;
        dest.y -= self.y;

        let xnew = dest.x * c - dest.y * s;
        let ynew = dest.x * s + dest.y * c;

        dest.x = xnew + self.x;
        dest.y = ynew + self.y;
        engine.render_line(orig, dest, [0.0, 1.0, 0.0, 1.0]);
        /////////////////////////////////
        // for rotations. It's useful////
        /////////////////////////////////

        // TODO TAKE NOTES
        // render line is using a different projection. That means that positions of the objects needs to be bounded
        // to the projection (which can be, ortho or persp).

        // At least with the ortho projection it is possible to change where the coordinate (0, 0) will be located.
        //
        // If I use what Cherno used when I place an object at position (0, 0) its center will be at the middle of the screen.
        // let proj = Mat4::orthographic_lh(-ar, ar, -1.0, 1.0, -1.0, 1.0);
        // This is confirmed

        // If I then use what I've been using. The object's center will be at the bottom left of the screen.
        // LearnOpenGL uses the same convention as I, and I believe pretty much every other framework out there.

        // Conclusion, this has been good because I was able to understand a new concept. Regarding the implementation
        // I should stick to using what I have been using as it seems to be the defacto standard. At least for 2D frameworks.
        // TODO TAKE NOTES

        // let line_color2: [f32; 4] = [1.0, 0.0, 0.0, 0.1];
        // let mut origcopy = glam::Vec3::new(self.x - self.scale_x / 2.0, self.y, 0.0);
        // let mut destcopy = glam::Vec3::new(self.x + self.scale_x / 2.0, self.y, 0.0);
        // let orig2 = (glam::Mat4::from_translation(glam::vec3(-self.x, -self.y, 0.0)) * glam::Mat4::from_rotation_z(rotation) * glam::Mat4::from_translation(glam::vec3(self.x, self.y, 0.0))).transform_point3(origcopy);
        // let dest2 = (glam::Mat4::from_translation(glam::vec3(-self.x, -self.y, 0.0)) * glam::Mat4::from_rotation_z(rotation) * glam::Mat4::from_translation(glam::vec3(self.x, self.y, 0.0))).transform_point3(destcopy);
        // engine.prepare_line_data(orig2, dest2, line_color2);

        // let orig = glam::Vec3::new(self.x, self.y + self.scale_y / 2.0, 0.0);
        // let dest = glam::Vec3::new(self.x, self.y - 100.0, 0.0);
        // let line_color: [f32; 4] = [1.0, 1.0, 0.0, 0.1];
        // engine.prepare_line_data(orig, dest, line_color)

        // CIRCLE
        let circle_position = vec3(100.0, 100.0, 0.0);
        let circle_scale: Vec3 = vec3(700.0, 200.0, 0.0);
        let circle_color: [f32; 4] = [1.0, 0.5, 0.3, 1.0];
        let thickness = 1.00; // from 0.01 (nothing inside, almost 1px border) to 1.0 (full)
        let fade = 0.009; // 0.0001 to 2.0, 0 being no fade. 0.009 makes it look smooth enough.
        engine.render_circle(circle_position, circle_scale, thickness, fade, circle_color);

        // EMPTY RECT
        // la posicion representa el centro del rect. ver por que no me andaba antes y ahora si...
        engine.render_rect(
            vec3(500.0, 300.0, 0.0),
            vec3(130.0, 130.0, 0.0),
            0.0,
            [1.0, 0.0, 0.0, 1.0],
        );

        engine.render_rect(
            vec3(300.0, 300.0, 0.0),
            vec3(130.0, 130.0, 0.0),
            angle,
            [1.0, 0.0, 0.0, 1.0],
        );
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
            x: 500.0,
            y: 300.0,

            amount_up: 0.0,
            amount_down: 0.0,
            amount_left: 0.0,
            amount_right: 0.0,

            scale_x: 130.0,
            scale_y: 130.0,
            speed: 5.0,
        };
        Self { container, player }
    }

    fn create_enemies(container: &mut EnemyContainer) {
        let color1: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
        let color2: [f32; 4] = [1.0, 0.0, 0.0, 0.3];
        let circle_color: [f32; 4] = [1.0, 0.0, 0.0, 0.3];

        let enemy1 = Enemy::new(200.0, 300.0, color1, Some("Cube 1"), String::from("tree"));
        let enemy2 = Enemy::new(300.0, 200.0, color2, Some("Cube 2"), String::from("tree"));
        let enemy_pika = Enemy::new(500.0, 500.0, color1, Some("Cube 2"), String::from("pika"));
        // let circle_enemy = CircleEnemy::new(100.0, 100.0, circle_color);

        container.add_enemy(enemy1);
        container.add_enemy(enemy2);
        container.add_enemy(enemy_pika);
        // container.add_enemy(circle_enemy);
    }
}

impl<'a> bm::Application for App<'a> {
    fn on_setup(&mut self, engine: &mut bm::Engine) {
        engine.create_texture(String::from("tree"), "src/happy-tree.png");
        engine.create_texture(String::from("pika"), "src/default.png");
        engine.create_texture(String::from("sims"), "src/sims.png");
        engine.create_texture(String::from("dvd"), "src/power-dvd.jpg");
        engine.create_texture(String::from("pumpkin"), "src/pumpkin.png");
    }

    fn on_update(&mut self, engine: &mut bm::Engine, delta_time: f32, time: f32) {
        self.player.update(engine, delta_time);
        self.container.on_update(engine, &self.player, delta_time);
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
