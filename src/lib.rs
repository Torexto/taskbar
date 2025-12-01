mod shortcut;
mod running_windows_app;

use crate::shortcut::{Icon, Shortcut};
use iced::widget::Image;
use iced::widget::button::{Status, Style};
use iced::window::Level;
use iced::*;
use std::process::Command;
use widget::{Row, button, container, image, text};

#[derive(Debug, Clone, Copy, Default)]
pub enum Position {
    Top,
    #[default]
    Bottom,
}

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub position: Position,
    pub size: u16,
    pub monitor_resolution: (u16, u16),
}

#[derive(Default)]
struct State {}

#[derive(Debug, Clone)]
pub enum Message {
    ShortcutClicked(Shortcut),
}

pub fn run(options: Options) -> Result {
    let app = application("taskbar", State::update, State::view)
        .decorations(false)
        .transparent(true)
        .level(Level::AlwaysOnTop)
        .window_size((options.monitor_resolution.0 as f32, options.size as f32));

    let app = match options.position {
        Position::Top => app.position(window::Position::Specific(Point::new(0f32, 0f32))),
        Position::Bottom => app.position(window::Position::Specific(Point::new(
            0f32,
            (options.monitor_resolution.1 - options.size) as f32,
        ))),
    };

    app.run()
}

fn taskbar_button_style(theme: &Theme, status: Status) -> Style {
    Style {
        background: None,
        ..Default::default()
    }
}

impl State {
    pub fn view(&self) -> Element<Message> {
        let mut row = Row::new().spacing(10);

        let elements = shortcut::read_taskbar_elements().unwrap_or_default();

        for element in elements.iter() {
            let element_clone = element.clone();
            if let None = element_clone.icon_path {
                dbg!(&element_clone.name);
            }
            let content: Element<Message> = match &element_clone.icon_path {
                Some(icon) => match icon {
                    Icon::Path(path) => image(path).into(),
                    Icon::Image(image) => {
                        let h = image::Handle::from_bytes(image.to_vec());
                        Image::new(h).into()
                    }
                },
                None => text(element_clone.name.to_owned()).into(),
            };
            let btn = button(content)
                .height(30)
                .style(taskbar_button_style)
                .on_press(Message::ShortcutClicked(element_clone));
            let btn = container(btn);
            row = row.push(btn);
        }

        let container = container(row).width(Fill).center_x(Fill);

        container.into()
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::ShortcutClicked(shortcut) => {
                println!("{:?} {:?}", shortcut.target, shortcut.args);
                Command::new(shortcut.target)
                    .args(shortcut.args.split_whitespace())
                    .spawn()
                    .expect("Failed to execute shortcut target");
            }
        }
    }
}

#[test]
fn name() {
    let shortcuts = shortcut::read_taskbar_elements().unwrap();

    for shortcut in shortcuts.iter() {
        assert!(shortcut.name.len() > 1);
    }
}

#[test]
fn target() {
    let shortcuts = shortcut::read_taskbar_elements().unwrap();

    for shortcut in shortcuts.iter() {
        let exist = shortcut.target.exists();
        if !exist {
            eprintln!("Target not found: {:?}", shortcut.target);
            eprintln!("Name: {:?}", shortcut.name);
        }
        assert!(exist);
    }
}

#[test]
fn icon() {
    let shortcuts = shortcut::read_taskbar_elements().unwrap();

    for shortcut in shortcuts.iter() {
        shortcut
            .icon_path
            .to_owned()
            .expect(format!("Icon path is empty for: {}", shortcut.name).as_str());
    }
}
