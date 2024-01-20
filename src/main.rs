#![allow(unused, dead_code)]
use winit::event::{Event, VirtualKeyCode, WindowEvent};

use bm::async_runner;
#[derive(Default)]
struct CubeContainer<'a> {
    cubes: Vec<bm::Cube<'a>>,
}

impl<'a> CubeContainer<'a> {
    pub fn update(&mut self, engine: &mut bm::Engine) {
        // move in random positions
        for cube in self.cubes.iter_mut() {
            cube.update(engine, (1.0, 1.0));
        }
    }

    pub fn add_cube(&mut self, elem: bm::Cube<'a>) {
        self.cubes.push(elem)
    }
}

#[derive(Default, Debug)]
struct Player {
    x: f32,
    y: f32,
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
}

#[derive(Default)]
struct App<'a> {
    player: Player,
    container: CubeContainer<'a>,
}

impl<'a> App<'a> {
    pub fn new() -> Self {
        let mut container = CubeContainer::default();
        let cubes = App::create_cubes(&mut container);
        Self {
            ..Default::default()
        }
    }
    fn create_cubes(container: &mut CubeContainer) {
        let cube1 = bm::Cube::new(32.0, 13.0, Some("Cube 1"));
        let cube2 = bm::Cube::new(32.0, 13.0, Some("Cube 1"));
        container.add_cube(cube1);
        container.add_cube(cube2);
    }
}

impl<'a> bm::Application for App<'a> {
    fn on_update(&mut self, engine: &mut bm::Engine) {
        self.player.update(engine);
        self.container.update(engine);
    }

    fn on_render(&mut self, engine: &mut bm::Engine) {
        let color: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
        // engine.begin_render(|render_pass| {
        //     engine.draw_quad(color, render_pass);
        //     // engine.draw_quad(color, &mut render_pass);
        // });

        /*
         engine.begin_render()

         engine.draw_quad(color, &mut render_pass);
         engine.draw_quad(color, &mut render_pass);
         engine.draw_quad(color, &mut render_pass);

         engine.end_render()

        */

        // dbg!("render method of app");
    }

    fn on_event(&mut self, engine: &mut bm::Engine, event: bm::MyEvent) {
        self.player.on_event(event);
    }
}

pub fn main() {
    let mut app = App::new();
    pollster::block_on(async_runner(app));
}
