use bm::async_runner;
use glam::{vec2, vec3, Vec2, Vec2Swizzles, Vec3, vec4};

use rand::random;
use rand::Rng;

/*
- API for `render_circle` is akward
- Can't change the clear color from the game.

 */

const WINDOW_WIDTH: f32 = 800.0;
const WINDOW_HEIGHT: f32 = 600.0;

const PIXELS_PER_METER: f32 = 50.0;
#[derive(Default)]
struct Particle {
    pos: Vec2,
    vel: Vec2,
    accel: Vec2,
    radius: f32,
    mass: f32,
    inv_mass: f32,
    forces: Vec2,
}

impl Particle {
    pub fn add_force(&mut self, force: Vec2) {
        self.forces += force
    }

    pub fn clear_forces(&mut self) {
        self.forces = Vec2::default();
    }

    pub fn integrate(&mut self, delta_time: f32) {
        // F = m * a
        // a = F / m
        // self.accel += self.forces * self.inv_mass * delta_time;
        self.accel = self.forces * self.inv_mass;
        self.vel += self.accel * delta_time;
        self.pos += self.vel * delta_time;
    }

    pub fn check_collisions(&mut self) {
        let mut pos = self.pos;
        let radius = self.radius;

        if pos.y - radius < 0.0 {
            self.pos.y = radius;
            self.vel.y *= -0.98;
        }

        if pos.y + radius > WINDOW_HEIGHT {
            self.pos.y = WINDOW_HEIGHT - radius;
            self.vel.y *= -0.98;
        }

        if pos.x + radius > WINDOW_WIDTH {
            self.pos.x = WINDOW_WIDTH - radius;
            self.vel.x *= -0.98;
        }
        if pos.x - radius < 0.0 {
            self.pos.x = radius;
            self.vel.x *= -0.98;
        }
    }
}

impl Particle {
    pub fn new(pos: Vec2, mass: f32, radius: f32) -> Self {
        Self {
            pos,
            vel: Vec2::default(),
            accel: Vec2::default(),
            mass,
            inv_mass: 1.0 / mass,
            radius,
            forces: Vec2::default(),
        }
    }
}

struct App {
    particles: Vec<Particle>,
}
impl bm::Application for App {
    fn on_setup(&mut self, engine: &mut bm::Engine) {
        let mut rng = rand::thread_rng();

        for _i in 0..5000 {
            let x = rng.gen_range(0.0..800.0);
            let y = rng.gen_range(0.0..600.0);
            let mass = rng.gen_range(2.0..4.0);
            self.particles
                .push(Particle::new(vec2(x, y), mass, mass * 3.0));
        }
        //self.particles.push(Particle::new(vec2(20.0, 500.0), 1.0, 4.0));
        //self.particles.push(Particle::new(vec2(50.0, 500.0), 3.0, 12.0));
    }

    fn on_update(&mut self, engine: &mut bm::Engine, delta_time: f32, time: f32) {
        let mut rng = rand::thread_rng();

        let thickness = 1.00; // from 0.01 (nothing inside, almost 1px border) to 1.0 (full)
        let fade = 0.019; // 0.0001 to 2.0, 0 being no fade. 0.009 makes it look smooth enough.
        for particle in self.particles.iter_mut() {
            // wind
            particle.add_force(vec2((0.3_f32.cos() * PIXELS_PER_METER  * 6.0 ), 0.0));

            // gravity
            particle.add_force(vec2(0.0, (-9.8_f32.sin() * PIXELS_PER_METER * 2.0 * particle.mass)));

            particle.integrate(delta_time);
            particle.check_collisions();

            particle.clear_forces();

            let x = 0.1 + 0.5 * (time ).cos();
            let y = 0.1 + 0.5 * (time + 1.0).cos();
            let z = 0.1 + 0.5 * (time + 2.0).cos();


            engine.render_circle(
                vec3(particle.pos.x, particle.pos.y, 0.0),
                vec3(particle.radius, particle.radius, 0.0),
                thickness,
                fade,
                vec4(x, y, z, 1.0).to_array(),
            );
        }
    }

    fn on_render(&mut self, engine: &mut bm::Engine) {

        //for particle in self.particles.iter() {
        //}
    }

    fn on_event(&mut self, engine: &mut bm::Engine, event: bm::MyEvent) {}
}
pub fn main() {
    let app = App {
        particles: Vec::default(),
    };
    pollster::block_on(async_runner(app));
}
