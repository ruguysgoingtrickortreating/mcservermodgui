use iced::{Border, Color, Element, Font, Task, font, widget::{Svg, button, column, container, space, svg, table, text}};
use rand::seq::IteratorRandom;
use smart_default::SmartDefault;

use crate::{Message, MinecraftVersion, ProgramData, STATIC_IMAGES, states::mod_downloader::ModDownloaderState};

#[derive(Clone, Debug)]
pub enum MainMessage {
    OpenButtonPressed,
}

pub struct MainState {
    program_data: ProgramData,
    mc_versions: Vec<MinecraftVersion>,
    pub mod_downloader_state: Option<ModDownloaderState>,

    include_snapshots: bool,
}
impl MainState {
    pub fn update(&mut self, _message: MainMessage) -> Task<Message> {
        match _message {
            MainMessage::OpenButtonPressed => {
                return Task::done(Message::OpenWindow(crate::WindowType::ModDownload))
            },
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
            button("open").on_press(MainMessage::OpenButtonPressed),
            space().height(40),
            table(columns, [STATIC_IMAGES.modrinth.clone()])
        ].padding(20).into()
        
    }

    pub fn new(program_data: ProgramData,mc_versions: Vec<MinecraftVersion>) -> Self {
        MainState {
            program_data,
            mc_versions,
            mod_downloader_state: None,
            include_snapshots: Default::default(),
        }
    }
}
