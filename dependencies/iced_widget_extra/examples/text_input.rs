use iced_widget_extra::text_input;

use iced::Element;
use iced::widget::center;

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view).run()
}

#[derive(Default)]
struct App {
    value: String,
}

#[derive(Debug, Clone)]
enum Message {
    UpdateValue(String),
    InputFocused,
    InputBlurred,
    InputEscaped,
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::UpdateValue(value) => {
                self.value = value;
            }
            Message::InputFocused => {
                dbg!("Input focused");
            }
            Message::InputBlurred => {
                dbg!("Input blurred");
            }
            Message::InputEscaped => {
                dbg!("Input escaped");
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        center(
            text_input("Enter value...", &self.value)
                .width(300)
                .on_focus(Message::InputFocused)
                .on_blur(Message::InputBlurred)
                .on_escape(Message::InputEscaped)
                .on_input(Message::UpdateValue),
        )
        .into()
    }
}
