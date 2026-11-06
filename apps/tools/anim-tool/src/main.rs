mod app;
mod atlas_generator;
mod camera;
mod common_state;
mod designer_state;
pub mod render;
mod showcase_state;
mod text_layout;

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
