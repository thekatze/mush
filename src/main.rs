use mush::application::Application;
use pollster::FutureExt;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    Application::build().block_on().expect("init failed").run();
}
