use iced_widget_extra::{action_area, text_input};

use iced::mouse::Interaction;
use iced::widget::{button, center, container, operation, row, text};
use iced::{Element, Length, Task, Theme};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .theme(App::theme)
        .run()
}

#[derive(Default)]
struct App {
    light_theme: bool,
    value: String,
    editing: bool,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleDarkLight,
    StartEditing,
    StopEditing,
    UpdateValue(String),
}

impl App {
    fn new() -> Self {
        Self {
            light_theme: false,
            value: String::from("Click to start editing"),
            editing: false,
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ToggleDarkLight => {
                self.light_theme = !self.light_theme;
            }
            Message::StartEditing => {
                self.editing = true;
                return operation::focus("input");
            }
            Message::StopEditing => self.editing = false,
            Message::UpdateValue(new_value) => self.value = new_value,
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        center(
            container(
                row![
                    if self.editing {
                        Element::from(
                            text_input("Value...", &self.value)
                                .id("input")
                                .on_blur(Message::StopEditing)
                                .on_submit(Message::StopEditing)
                                .on_input(Message::UpdateValue),
                        )
                    } else {
                        Element::from(
                            action_area(text(&self.value))
                                .on_press(Message::StartEditing)
                                .interaction(Interaction::Text)
                                .style(editable_text)
                                .padding(5)
                                .width(Length::Fill),
                        )
                    },
                    button("Dark/light").on_press(Message::ToggleDarkLight)
                ]
                .spacing(10),
            )
            .width(300),
        )
        .into()
    }

    fn theme(&self) -> Theme {
        if self.light_theme {
            Theme::Light
        } else {
            Theme::Dark
        }
    }
}

fn editable_text(theme: &Theme, status: action_area::Status) -> action_area::Style {
    let palette = theme.extended_palette();

    let base = action_area::Style {
        text_color: palette.background.base.text,
        ..action_area::Style::default()
    };

    match status {
        action_area::Status::Active | action_area::Status::Pressed => base,
        action_area::Status::Hovered => action_area::Style {
            background: Some(palette.background.weak.color.into()),
            ..base
        },
        action_area::Status::Disabled => action_area::disabled(base),
    }
}
