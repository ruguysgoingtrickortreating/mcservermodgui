use iced::alignment::{Horizontal, Vertical};
use iced::font::Weight;
use iced_widget::{center_x, center_y, checkbox, scrollable, text};
use iced_widget_extra::table;
use iced_widget_extra::table::EdgePadding;

use iced::{Color, Element, Font};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view).run()
}

struct App {
    persons: Vec<Person>,
}

#[derive(Debug, Clone)]
enum Message {}

impl App {
    fn new() -> Self {
        Self {
            persons: Person::large_list(),
        }
    }

    fn update(&mut self, message: Message) {
        match message {}
    }

    fn view(&self) -> Element<'_, Message> {
        let table = {
            let bold = |header| {
                text(header).font(Font {
                    weight: Weight::Bold,
                    ..Font::DEFAULT
                })
            };

            let columns = [
                table::column(bold("#"), |(index, _person): (usize, &Person)| {
                    text(format!("#{}", index + 1))
                })
                .header_align_x(Horizontal::Right),
                table::column(bold("Name"), |(_index, person): (usize, &Person)| {
                    text(&person.name).line_height(4.0)
                }),
                table::column(bold("Active"), |(_index, person): (usize, &Person)| {
                    checkbox(person.active)
                })
                .content_align_x(Horizontal::Right)
                .content_align_y(Vertical::Bottom),
            ];

            table(columns, self.persons.iter().enumerate())
                .padding(10)
                .edge_padding(EdgePadding::none())
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

#[derive(Clone, Debug)]
struct Person {
    name: String,
    active: bool,
}

impl Person {
    fn large_list() -> Vec<Self> {
        #[rustfmt::skip]
        let first_names = [
            "Alice", "Bob", "Charlie", "Diana", "Edward", "Fiona", "George", "Helen", "Ivan",
            "Julia", "Kevin", "Luna", "Michael", "Nina", "Oscar", "Paula", "Quinn", "Rachel",
            "Samuel", "Tina", "Ulrich", "Victoria", "Walter", "Xara", "Yuri", "Zoe",
        ];

        #[rustfmt::skip]
        let last_names = [
            "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller",
            "Davis", "Rodriguez", "Martinez", "Hernandez", "Lopez", "Gonzalez",
            "Wilson", "Anderson", "Thomas", "Taylor", "Moore", "Jackson", "Martin",
        ];

        (0..500)
            .map(|i| Person {
                name: format!(
                    "{} {}",
                    first_names[i % first_names.len()],
                    last_names[(i / first_names.len()) % last_names.len()]
                ),
                active: i % 3 != 0,
            })
            .collect()
    }
}
