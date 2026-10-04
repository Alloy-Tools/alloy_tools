mod app;
mod camera;
mod render;
mod state;

pub use camera::Camera;

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    pollster::block_on(future)
}

fn main() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("{info}");
        eprintln!("\nPress Enter to exit...");
        let _ = std::io::stdin().read_line(&mut String::new());
    }));

    app::App::new().run().unwrap()
}
