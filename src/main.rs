use core::fmt;
use std::{cell::OnceCell, collections::HashMap, sync::{Arc, LazyLock, OnceLock}};

use iced::{Color, Element, Task, advanced::{image::Handle as RasterHandle, svg::Handle as SvgHandle}, widget::{button, checkbox, column, pick_list}, window};
use reqwest::Client;

mod states;
mod util;
use states::*;
use util::*;

use mod_downloader::{ModDownloaderState,ModDownMsg};
use setup::{SetupState,SetupMessage};
use states::{init::InitState,main_window::MainState};

use crate::states::{init::InitMessage, main_window::MainMessage};

struct _StaticImages {
    missing: RasterHandle,
    unknown: RasterHandle,
    modrinth: SvgHandle,
    curseforge: SvgHandle,
    hangar: SvgHandle,
    file: SvgHandle
}

static REQ_CLIENT: LazyLock<Client> = LazyLock::new(|| Client::new());
static STATIC_IMAGES: LazyLock<_StaticImages> = LazyLock::new(|| _StaticImages {
    missing: RasterHandle::from_bytes(include_bytes!("../assets/missing_image.png").to_vec()),
    unknown: RasterHandle::from_bytes(include_bytes!("../assets/unknown_image.png").to_vec()),
    modrinth: SvgHandle::from_memory(include_bytes!("../assets/modrinth.svg")),
    curseforge: SvgHandle::from_memory(include_bytes!("../assets/curseforge.svg")),
    hangar: SvgHandle::from_memory(include_bytes!("../assets/hangar.svg")),
    file: SvgHandle::from_memory(include_bytes!("../assets/folder.svg")),
});


#[derive(serde::Serialize,serde::Deserialize,Debug)]
struct ProgramData {
    name: String,
    loader: ModLoader,
    version: String,
}


#[derive(serde::Serialize,serde::Deserialize, Debug)]
struct MinecraftVersion {
    id: String,
    #[serde(rename="type",deserialize_with="_version_kind_handler")]
    kind: VersionKind
}

#[derive(serde::Serialize,serde::Deserialize,strum_macros::Display,strum_macros::VariantArray,Clone, Copy, PartialEq, Debug)]
enum ModLoader {
    Fabric,
    NeoForge,
    Forge,
    Paper,
    Purpur,
    Folia,
    Velocity,
}

#[derive(serde::Serialize,Debug)]
enum VersionKind {
    Release,
    Snapshot,
    OldBeta,
    OldAlpha
}


#[derive(Clone)]
enum ImageType {
    Svg(SvgHandle),
    Raster(RasterHandle)
}



fn main() -> iced::Result {
    // let (mc_versions, program_data) = init::init().map_err(|e| panic!("error in init: {e}")).unwrap();

    // MC_VERSIONS.set(mc_versions).unwrap();

    iced::daemon(
        || {
            let app = AppState::default();

            (app,Task::done(Message::OpenWindow(WindowType::Init)))
        },AppState::update,
        AppState::view
    )
        // .theme(iced::Theme::Custom(Arc::new(iced::theme::Custom::new("".to_string(), iced::theme::Palette {
        //     primary: Color::TRANSPARENT,
        //     ..iced::theme::Palette::DARK
        // })))).antialiasing(true)
        .theme(iced::Theme::KanagawaDragon)
        .subscription(|_| window::close_events().map(Message::WindowClosed))
        .run()
}

#[derive(Debug,Clone)]
enum Message {
    OpenLink(String),
    OpenWindow(WindowType),

    WindowOpened,
    WindowClosed(window::Id),

    InitMessage(InitMessage),
    SetupMessage(SetupMessage),
    MainMessage(MainMessage),
    ModDLMessage(ModDownMsg),
}

#[derive(Default)]
struct AppState {
    windows: HashMap<window::Id, Window>,

    init_state: Option<InitState>,
    setup_state: Option<SetupState>,
    main_state: Option<MainState>,
    mod_downloader_state: Option<ModDownloaderState>,
}
impl AppState {
    fn update(&mut self, _message:Message) -> Task<Message> {
        match _message {
            Message::OpenLink(url) => {println!("opening {url}");let _ = webbrowser::open(url.as_str());},
            Message::OpenWindow(win_type) => {
                println!("#openingwindow #gay");
                let task: Task<Message>;
                let settings = window::Settings::default();
                match win_type {
                    WindowType::ModDownload => {
                        if self.mod_downloader_state.is_some() {panic!("Tried to open ModDownloader window while ModDownloader state already exists")};

                        let mut state = ModDownloaderState::default();
                        task = state.init();
                        self.mod_downloader_state = Some(state);
                    }
                    WindowType::Init => {
                        if self.init_state.is_some() {panic!("Tried to open Init window while Init state already exists")};

                        let mut state = InitState::default();
                        task = state.init().map(|v| Message::InitMessage(v));
                        self.init_state = Some(state);
                    }
                    WindowType::Setup => {
                        // println!("made me a setup 😋");
                        // // if self.setup_state.is_some() {panic!("Tried to open Setup window while Setup state already exists")};

                        // let state = SetupState::default();
                        // task = Task::none();
                        // self.setup_state = Some(state);
                        panic!("setup should not be opened on its own")
                    },
                    WindowType::Main => {
                        println!("made me a main 😋");
                        todo!()
                    },
                };
                let (id, open) = window::open(settings);
                let win = Window::new(win_type);
                self.windows.insert(id, win);
                return Task::batch([open.map(|_| Message::WindowOpened),task])
            }
            Message::WindowOpened => (),
            Message::WindowClosed(id) => {
                let w = self.windows.remove(&id).unwrap();
                match w.window_type {
                    WindowType::ModDownload => {
                        self.mod_downloader_state = None;
                    }
                    _ => {
                        return iced::exit();
                    }
                }
            }
            Message::InitMessage(m) => {
                if let InitMessage::InitConcluded = m {
                    let id = self.windows.iter().find_map(|(&id,w)| if w.window_type == WindowType::Init {Some(id)} else {None}).expect("tried to close an Init window that didn't exist");   
                    let mut state = self.init_state.take().unwrap();
                    let versions = state.versions_list.unwrap().versions;
                    let kind: WindowType;
                    
                    println!("removing id...");

                    if let Some(program_data) = state.program_data.take() {
                        kind = WindowType::Main;
                        let main = MainState::new(program_data,versions);
                        self.main_state = Some(main);
                    } else {
                        kind = WindowType::Setup;
                        let mut setup = SetupState::default();
                        if let Some(v) = &state.assumed_version && versions.iter().find(|s| &s.id == v).is_some() {
                            setup.selected_version = state.assumed_version;
                        }
                        setup.name = state.assumed_name;
                        setup.mc_versions = versions;
                        setup.selected_loader = state.assumed_loader;
                        self.setup_state = Some(setup);
                    }
                    self.windows.insert(id, Window{window_type: kind});
                    // return Task::batch([window::close(id),Task::done(Message::OpenWindow(kind))]);
                    return Task::none()
                }

                return self.init_state.as_mut().unwrap().update(m).map(|v|Message::InitMessage(v));
            }
            Message::SetupMessage(m) => {
                if let SetupMessage::SetupConcluded = m {
                    let id = self.windows.iter().find_map(|(&id,w)| if w.window_type == WindowType::Setup {Some(id)} else {None}).expect("tried to close an Setup window that didn't exist");   
                    let mut state = self.setup_state.take().unwrap();
                    let versions = state.mc_versions;

                    let main = MainState::new(state.program_data.take().expect("received unfinished program data"),versions);
                    self.main_state = Some(main);
                    self.windows.insert(id, Window{window_type: WindowType::Main});
                    return Task::none();
                }
                return self.setup_state.as_mut().unwrap().update(m)
            }
            Message::MainMessage(m) => return self.main_state.as_mut().unwrap().update(m),
            Message::ModDLMessage(m)  => return self.mod_downloader_state.as_mut().unwrap().update(m),
        }
        Task::none()
    }

    fn view(&'_ self, window_id: window::Id) -> Element<'_, Message> {
        match self.windows.get(&window_id).expect(&format!("{window_id} not in windows list")).window_type {
        WindowType::Init => self.init_state.as_ref().unwrap().view().map(|v| Message::InitMessage(v)),
        WindowType::Setup => self.setup_state.as_ref().unwrap().view().map(|v| Message::SetupMessage(v)),
        WindowType::Main => self.main_state.as_ref().unwrap().view().map(|v| Message::MainMessage(v)),
        WindowType::ModDownload => self.mod_downloader_state.as_ref().unwrap().view().map(|v| Message::ModDLMessage(v)),
    }     
    }
}

pub enum ProgramPhase {
    Init(InitState),
    Setup(SetupState),
    Main(MainState)
}

#[derive(Debug,Clone,Copy,PartialEq)]
pub enum WindowType {
    Init,
    Setup,
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



fn _version_kind_handler<'de, D>(
    deserializer: D,
) -> Result<VersionKind, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    Ok(match s.as_str() {
        "release" | "Release" => VersionKind::Release,
        "snapshot" | "Snapshot" => VersionKind::Snapshot,
        "old_beta" | "OldBeta" => VersionKind::OldBeta,
        "old_alpha" | "OldAlpha" => VersionKind::OldAlpha,
        _ => panic!("invalid response to version type: {s}")
    })
}