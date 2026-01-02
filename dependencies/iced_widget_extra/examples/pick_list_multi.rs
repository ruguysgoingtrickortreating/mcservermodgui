use iced_widget_extra::pick_list_multi;
use iced_widget_extra::pick_list_multi::{SelectionState, update_selection};

use iced::widget::text::Shaping;
use iced::widget::{center, column};

use iced::{Element, Font, Length};

fn main() -> iced::Result {
    iced::application(App::default, App::update, App::view).run()
}

#[derive(Default)]
struct App {
    languages: Vec<(Option<Language>, SelectionState)>,
}

#[derive(Debug, Clone)]
enum Message {
    LanguageSelected((Option<Language>, SelectionState)),
}

impl App {
    fn update(&mut self, message: Message) {
        match message {
            Message::LanguageSelected((language, state)) => {
                update_selection(&mut self.languages, language, state);
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        center(
            column![
                pick_list_multi(Language::ALL, &self.languages, Message::LanguageSelected)
                    .optional(false)
                    .width(Length::Fill)
                    .text_shaping(Shaping::Advanced),
                pick_list_multi(Language::ALL, &self.languages, Message::LanguageSelected)
                    .selection_label(format!("{} items selected", self.languages.len()))
                    .optional(true)
                    .none_label("None")
                    .none_separator(true)
                    .width(Length::Fill)
                    .font(Font::MONOSPACE)
                    .text_shaping(Shaping::Advanced),
                pick_list_multi(Language::ALL, &self.languages, Message::LanguageSelected)
                    .symbols(["", "+", "-"])
                    .optional(true)
                    .none_label("None")
                    .none_separator(true)
                    .width(Length::Fill)
                    .exclusion_mode(true)
                    .text_shaping(Shaping::Advanced),
            ]
            .width(600)
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
