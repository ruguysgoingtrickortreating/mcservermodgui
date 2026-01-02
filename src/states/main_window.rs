use iced::{Border, Color, Element, Font, Task, font, widget::{Svg, button, column, container, space, svg, table, text}};
use iced::widget::pick_list;
use rand::seq::IteratorRandom;
use smart_default::SmartDefault;

use crate::{Message, MinecraftVersion, ProgramData, STATIC_IMAGES, states::mod_downloader::ModDownloaderState};
use crate::util::icon_pick_list::icon_pick_list;

#[derive(Clone, Debug)]
pub enum MainMessage {
    OpenButtonPressed,
    TestSelected(String),
}

pub struct MainState {
    pub program_data: ProgramData,
    pub mod_downloader_state: Option<ModDownloaderState>,

    include_snapshots: bool,
    selected_gem: String,
}
impl MainState {
    pub fn update(&mut self, _message: MainMessage) -> Task<Message> {
        match _message {
            MainMessage::OpenButtonPressed => {
                return Task::done(Message::OpenWindow(crate::WindowType::ModDownload))
            },
            MainMessage::TestSelected(s) => {
                self.selected_gem = s;
            }
        };
        Task::none()
    }

    pub fn view(&self) -> Element<MainMessage> {
        let columns = [
            table::column("Mustard", |s: crate::SvgHandle| 
            // container(svg(s)).style(|t|container::Style::default().border(Border::default().width(1).color(Color::WHITE)))
            svg(s).width(30)//.height(30)
        )
        ];
        column![
            text(&self.program_data.name).font(Font {weight: font::Weight::Bold, ..Default::default()}).size(30),
            svg(STATIC_IMAGES.modrinth.clone()).style(|t,v|svg::Style { color: Some(Color::from_rgb8(27, 217, 106)) }),
            svg(STATIC_IMAGES.curseforge.clone()).style(|t,v|svg::Style { color: Some(Color::from_rgb8(255, 120, 77)) }),
            svg(STATIC_IMAGES.file.clone()).style(|t,v|svg::Style { color: Some(Color::from_rgb8(255, 200, 122)) }),
            svg(STATIC_IMAGES.filter.clone()),
            svg(STATIC_IMAGES.neoforge.clone()),
            button("open").on_press(MainMessage::OpenButtonPressed),
            space().height(40),
            table(columns, [STATIC_IMAGES.modrinth.clone()]),
            space().height(80),
            icon_pick_list(vec!["foid".to_string(),"chud".to_string(),"gem".to_string()],Some(&self.selected_gem), MainMessage::TestSelected,
                |s| match s.as_str() {
                    "chud" => vec![iced::advanced::svg::Svg::new(STATIC_IMAGES.modrinth.clone()), iced::advanced::svg::Svg::new(STATIC_IMAGES.curseforge.clone())],
                    _ => vec![iced::advanced::svg::Svg::new(STATIC_IMAGES.neoforge.clone())]
                },
            ),
            pick_list(vec!["foid".to_string(),"chud".to_string(),"gem".to_string()],Some(&self.selected_gem), MainMessage::TestSelected)
        ].padding(20).into()
        
    }

    pub fn new(program_data: ProgramData) -> Self {
        MainState {
            program_data,
            mod_downloader_state: None,
            include_snapshots: Default::default(),
            selected_gem: Default::default(),
        }
    }
}
