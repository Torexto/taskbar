use taskbar::{Options, run, Position};

fn main() -> iced::Result {
    let mut options = Options::default();
    options.size = 30;
    options.position = Position::Bottom;
    options.monitor_resolution = (1920, 1080);
    run(options)
}
