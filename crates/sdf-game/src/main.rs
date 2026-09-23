mod app;
mod render;
mod game;
mod map;

fn main() {
    app::App::new().run().unwrap()
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    pollster::block_on(future)
}
