use iced_widget::{center_x, center_y, column, scrollable, text, text_input};
use iced_widget_extra::pair_grid;

use iced::alignment::Horizontal;
use iced::{Color, Element};

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view).run()
}

#[derive(Default)]
struct App;

#[derive(Debug, Clone)]
enum Message {}

impl App {
    fn update(&mut self, message: Message) {
        match message {}
    }

    fn view(&self) -> Element<'_, Message> {
        let table = {
            let pairs = [
                pair_grid::pair(text("Tag:"), text("test")),
                pair_grid::pair(text("Modified at:"), text("today")),
                pair_grid::pair(text("Testing:"), text("testing!"))
                    .left_align_x(Horizontal::Right)
                    .right_align_x(Horizontal::Right),
                pair_grid::pair(
                    text("A moderate text:"),
                    column![text("A moderate not too long text"), text("BBBB")],
                ),
                pair_grid::pair(text("Label:"), text_input("placeholder", "value")),
            ];

            pair_grid(pairs).spacing(10)
        };

        center_y(
            scrollable(center_x(
                Element::from(table).explain(Color::from_rgb(0.0, 0.0, 1.0)),
            ))
            .spacing(10),
        )
        .padding(10)
        .into()
    }
}
