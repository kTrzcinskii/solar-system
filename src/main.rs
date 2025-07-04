use anyhow::Result;
use solar_system::app::App;
use winit::event_loop::EventLoop;

fn run() -> Result<()> {
    let event_loop = EventLoop::new()?;
    let mut app = App::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}

fn main() -> Result<()> {
    env_logger::init();
    run()
}
