use std::{collections::{HashMap, HashSet}, f32::consts::PI, sync::{Arc, LazyLock}};

use iced::{Alignment, Color, Degrees, Element, Gradient, Radians, Task, advanced::{image::Handle as RasterHandle, svg::Handle as SvgHandle, widget::operation}, color, gradient::Linear, widget::{self, Button, button, column, container, row, scrollable::{self, Viewport}, text, text_input}, window};
use frostmark::{MarkState, MarkWidget};
use reqwests::ImageData;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use smart_default::{self, SmartDefault};

mod mod_downloader;

use mod_downloader::{ModDownloader,ModDownMsg};

mod reqwests;

static REQ_CLIENT: LazyLock<Client> = LazyLock::new(|| Client::new());
static MISSING_IMAGE: LazyLock<RasterHandle> = LazyLock::new(|| RasterHandle::from_bytes(include_bytes!("../assets/missing_image.png").to_vec()));

fn main() -> iced::Result {
    iced::daemon("mcservermodgui",App::update,App::view)
    .theme(|_,_| iced::Theme::Custom(Arc::new(iced::theme::Custom::new("".to_string(), iced::theme::Palette {
        primary: Color::TRANSPARENT,
        ..iced::theme::Palette::DARK
    }))))
    .run_with(|| {
        let mut app = App::default();
        (app,Task::done(Message::OpenWindow(WindowType::Main)))
    })
}

#[derive(Debug,Clone)]
enum Message {
    OpenLink(String),
    OpenWindow(WindowType),

    WindowOpened,

    ModDLMessage(ModDownMsg),
}


#[derive(Clone)]
enum ImageType {
    Svg(SvgHandle),
    Raster(RasterHandle)
}

#[derive(SmartDefault)]
struct App {
    #[default(_code = "RasterHandle::from_bytes(include_bytes!(\"../assets/missing_image.png\").to_vec())")]
    missing_image: RasterHandle,
    windows: HashMap<window::Id, Window>,
    state_mod_dl: Option<ModDownloader>,

}
impl App {
    fn update(&mut self, _message:Message) -> Task<Message> {
        match _message {
            Message::OpenLink(url) => {let _ = webbrowser::open(url.as_str());},
            Message::OpenWindow(win_type) => {
                let task: Task<Message>;
                match win_type {
                    WindowType::ModDownload => {
                        if self.state_mod_dl.is_some() {return Task::none()}

                        let mut m = ModDownloader::default();
                        task = m.init();
                        self.state_mod_dl = Some(m);
                    }
                    _ => task = Task::none()
                };
                let (id, open) = window::open(window::Settings::default());
                let win = Window::new(win_type);
                self.windows.insert(id, win);
                return Task::batch([open.map(|_| Message::WindowOpened),task])
            }
            Message::WindowOpened => (),
            Message::ModDLMessage(m)  => return self.state_mod_dl.as_mut().unwrap().update(m)
        }
        Task::none()
    }

    fn view(&'_ self, window_id: window::Id) -> Element<'_, Message> {
        match self.windows.get(&window_id).unwrap().window_type {
        WindowType::Init => "".into(),
        WindowType::Main => {
            button("open diddy").on_press(Message::OpenWindow(WindowType::ModDownload)).into()
        },
        WindowType::ModDownload => {
            self.state_mod_dl.as_ref().unwrap().view().map(|v|Message::ModDLMessage(v))
        }
    }     
    }
}

#[derive(Debug,Clone,PartialEq)]
pub enum WindowType {
    Init,
    Main,
    ModDownload,
}

pub struct Window {
    pub window_type: WindowType
}

impl Window {
    pub fn new(window_type: WindowType) -> Self {
        Self { window_type}
    }
}