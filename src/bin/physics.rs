use bm::async_runner;

struct App;
impl bm::Application for App {
    fn on_setup(&mut self, engine: &mut bm::Engine) {}

    fn on_update(&mut self, engine: &mut bm::Engine, delta_time: f32) {}

    fn on_render(&mut self, engine: &mut bm::Engine) {}

    fn on_event(&mut self, engine: &mut bm::Engine, event: bm::MyEvent) {}
}
pub fn main() {
    let app = App;
    pollster::block_on(async_runner(app));
}
