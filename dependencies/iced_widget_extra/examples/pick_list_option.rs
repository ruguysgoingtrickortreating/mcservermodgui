use iced_widget_extra::pick_list_option;

use iced::Element;
use iced::widget::text::Shaping;
use iced::widget::{center, column};

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view).run()
}

#[derive(Default)]
struct App {
    language: Option<Language>,
}

#[derive(Debug, Clone)]
enum Message {
    LanguageSelected(Option<Language>),
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::LanguageSelected(language) => {
                self.language = language;
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        center(
            column![
                pick_list_option(Language::ALL, self.language, Message::LanguageSelected)
                    .optional(true)
                    .none_label("System")
                    .none_is_value(true)
                    .none_separator(true)
                    .text_shaping(Shaping::Advanced),
                pick_list_option(Language::ALL, self.language, Message::LanguageSelected)
                    .optional(true)
                    .text_shaping(Shaping::Advanced),
            ]
            .spacing(10),
        )
        .into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Language {
    English,
    German,
    Italian,
    French,
    Japanese,
}

impl Language {
    const ALL: &'static [Self] = &[
        Self::English,
        Self::German,
        Self::Italian,
        Self::French,
        Self::Japanese,
    ];
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::English => "English",
            Self::German => "Deutsch",
            Self::Italian => "Italiano",
            Self::French => "Français",
            Self::Japanese => "日本語",
        })
    }
}
