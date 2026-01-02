use iced_widget_extra::text_editor;
use iced_widget_extra::text_editor::focus;

use iced::widget::{button, center, column};
use iced::{Element, Task};

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view).run()
}

#[derive(Default)]
struct App {
    content: text_editor::Content,
}

#[derive(Debug, Clone)]
enum Message {
    ActionPerformed(text_editor::Action),
    InputFocused,
    InputBlurred,
    FocusEditor,
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ActionPerformed(action) => {
                dbg!(&action);
                self.content.perform(action);
            }
            Message::InputFocused => {
                dbg!("Input focused");
            }
            Message::InputBlurred => {
                dbg!("Input blurred");
            }
            Message::FocusEditor => {
                return focus("editor");
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        center(column![
            text_editor(&self.content)
                .id("editor")
                .width(300)
                .on_action(Message::ActionPerformed)
                .on_focus(Message::InputFocused)
                .on_blur(Message::InputBlurred),
            button("Focus editor").on_press(Message::FocusEditor)
        ])
        .into()
    }
}
