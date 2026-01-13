use std::{
    collections::{HashMap, HashSet},
    f32::consts::PI,
};
use std::fmt::Display;
use std::time::{Duration, Instant};
use frostmark::{MarkState, MarkWidget};
use iced::{Color, Element, Radians, Task, advanced::{image::Handle as RasterHandle, svg::Handle as SvgHandle}, gradient::Linear, widget::{
    self, button, column, container, row, scrollable, checkbox, image, mouse_area, space, stack, svg, Button,
    scrollable::Viewport,
    text, text_input,
}, border, mouse, Border, Font, font};
use iced::theme::palette::deviate;
use iced::widget::scrollable::Scrollbar;
use iced::widget::{center, hover, opaque, rich_text, right, rule, span, tooltip};
use iced::widget::button::Status;
use iced_widget_extra::pick_list_multi;
use iced_widget_extra::pick_list_multi::{update_selection, SelectionState};
use iced_selection;
use itertools::Itertools;
use rs_abbreviation_number::{AbbreviationOptions, NumericAbbreviate};
use serde::Deserialize;
use smart_default::{self, SmartDefault};
use strum::VariantArray;
use crate::{ImageType, STATIC_IMAGES, Message::{self, ModDLMessage as SuperMsg}, reqwests, MinecraftVersion, MC_VERSIONS, ProgramData, ModLoader, SVG_MOD_LOADERS, VersionKind, WindowType, bold, ModProvider};
use crate::util::{circular,icon_pick_list::{self, icon_pick_list, Catalog}};

mod modrinth;

#[derive(Debug, Clone)]
pub enum ModDownMsg {
    OpenLink(String),
    CloseRequested,

    ImageDownloaded(Result<reqwests::ImageData, String>),
    ModsSearchReceived(Result<Vec<u8>, String>),
    ModReceived(Result<Vec<u8>, String>),
    ModVersionsReceived(Result<(String, Vec<u8>), String>),
    DownloadVersionsReceived(Result<Vec<u8>, String>),
    CategoriesReceived(Result<Vec<u8>, String>),

    ProviderButtonPressed(ModProvider),

    ModListingPressed(usize),
    ModsListScrolled(Viewport),

    FilterButtonPressed,
    FilterVersionPicked((Option<MinecraftVersion>, SelectionState)),
    FilterLoaderPicked((Option<ModLoader>, SelectionState)),
    FilterCategoryPicked((Option<String>, SelectionState)),
    ShowSnapshotsChecked(bool),
    ServerSideModsChecked(bool),

    SearchTyped(String),
    SearchSubmitted,
    RetrySearchPressed,

    ModVersionPicked(ModVersion),
    SelectVersionButtonPressed,
    SelectedVersionTrashPressed(usize),
    DownloadButtonPressed,

    ConfirmCloseButtonPressed,
    CancelCloseButtonPressed,

    None,
}

#[derive(Default)]
enum PopupState {
    #[default]
    None,
    CloseConfirmation,
    DownloadConfirmation,
    NetworkError(&'static str, String),
}

#[derive(Default)]
enum FetchState {
    #[default]
    Done,
    Fetching,
    Errored
}

#[derive(Default)]
enum DownloadVerState {
    #[default]
    Fetching,
    Done(DownloadVerData)
}

struct DownloadVerData {

}

#[derive(Default)]
pub struct ModDownloaderState {
    popup_state: PopupState,

    current_provider: ModProvider,

    markup_state: MarkState,

    current_mod: Option<ModInfo>,
    cached_images: HashMap<String, ImageType>,
    cached_mods: HashMap<String, ModrinthMod>,
    images_queued: HashSet<String>,
    cached_categories: Vec<String>,

    selected_mod_versions: Vec<ModVersionQueued>,

    current_searchbar_text: String,
    current_query: String,
    mods_search_results: Vec<ModrinthSearchResult>,
    mods_search_offset: u64,

    show_filter_option: bool,
    show_snapshots: bool,
    server_sided_mods_only: bool,
    selected_filter_versions: Vec<(Option<MinecraftVersion>, SelectionState)>,
    selected_filter_loaders: Vec<(Option<ModLoader>, SelectionState)>,
    selected_filter_categories: Vec<(Option<String>, SelectionState)>,

    time_since_mod_button_clicked: Option<Instant>,
    scroll_load_debounce: bool,
    is_search_fetching: FetchState,
    search_fetching_sequence_number: usize, // used to prevent race conditions where a search is started before another one finishes
    is_mod_fetching: FetchState,
    is_download_versions_fetching: FetchState,
}
impl ModDownloaderState {
    pub fn update(&mut self, _message: ModDownMsg) -> Task<Message> {
        match _message {
            ModDownMsg::None => (),
            ModDownMsg::OpenLink(url) => return Task::done(Message::OpenLink(url)),
            ModDownMsg::CloseRequested => {
                if self.selected_mod_versions.is_empty() {
                    return Task::done(Message::CloseWindow(WindowType::ModDownload));
                } else {
                    self.popup_state = PopupState::CloseConfirmation;
                }
            }

            ModDownMsg::ImageDownloaded(res) => match res {
                Ok(img) => {
                    if img.is_svg {
                        self.cached_images
                            .insert(img.url, ImageType::Svg(SvgHandle::from_memory(img.bytes)));
                    } else {
                        self.cached_images.insert(
                            img.url,
                            ImageType::Raster(RasterHandle::from_bytes(img.bytes)),
                        );
                    }
                }
                Err(err) => {
                    eprintln!("Couldn't download image: {err}");
                }
            },
            ModDownMsg::ModsSearchReceived(res) => match res {
                Ok(val) => {
                    self.is_search_fetching = FetchState::Done;
                    let des = &mut serde_json::Deserializer::from_slice(&val);
                    let result: Result<ModrinthSearch, _> = serde_path_to_error::deserialize(des);
                    match result {
                        Ok(search) => {
                            let mut img_tasks: Vec<Task<Message>> = vec![];
                            // println!("{:?}",search);
                            for r in search.hits {
                                if let Some(url) = &r.icon_url {
                                    img_tasks.push(Task::perform(
                                        reqwests::download_image(url.clone()),
                                        |r| SuperMsg(ModDownMsg::ImageDownloaded(r)),
                                    ));
                                }
                                self.mods_search_results.push(r);
                            }
                            self.scroll_load_debounce = false;
                            return Task::batch(img_tasks);
                        }
                        Err(e) => {
                            let path = e.path().to_string();
                            eprintln!(
                                "error parsing JSON from search: {path}\ndumping json: {}",
                                String::from_utf8(val).unwrap_or_else(|e| e.to_string())
                            )
                        }
                    };
                }
                Err(err) => {
                    self.is_search_fetching = FetchState::Errored;
                    eprintln!("Couldn't get json: {err}");
                    self.set_popup_state(PopupState::NetworkError("Error searching for mods", err));
                }
            },
            ModDownMsg::ModVersionsReceived(res) => match res {
                Ok((id, val)) => {
                    let Some( current_mod) = self.current_mod.as_mut() else {
                        return Task::none()
                    };
                    if id != current_mod.id {return Task::none()}

                    let des = &mut serde_json::Deserializer::from_slice(&val);
                    let result: Result<Vec<ModrinthVersionMass>, _> = serde_path_to_error::deserialize(des);
                    match result {
                        Ok(versions) => {
                            let v = versions.into_iter().map(|m| ModVersion {
                                name: m.name,
                                id: m.id,
                                loaders: m.loaders.iter().filter_map(|x| match x.as_str() {
                                    "fabric" => Some(ModLoader::Fabric),
                                    "neoforge" => Some(ModLoader::NeoForge),
                                    "forge" => Some(ModLoader::Forge),
                                    "paper" => Some(ModLoader::Paper),
                                    "purpur" => Some(ModLoader::Purpur),
                                    "folia" => Some(ModLoader::Folia),
                                    "velocity" => Some(ModLoader::Velocity),
                                    _ => None
                                }).collect_vec(),
                            }).collect_vec();
                            current_mod.selected_version = v.get(0).map(|m|m.clone());
                            current_mod.cached_versions = v;
                        }
                        Err(err) => {
                            panic!("versions received error deserializing {err}")
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Couldn't get categories: {err}");
                    self.set_popup_state(PopupState::NetworkError("Error getting mod versions", err));
                }
            }
            ModDownMsg::DownloadVersionsReceived(res) => match res {
                Ok(val) => {
                    let des = &mut serde_json::Deserializer::from_slice(&val);
                    let result: Vec<ModrinthVersionDownload> = serde_path_to_error::deserialize(des).expect("error deserializing download versions");

                }
                Err(err) => {
                    eprintln!("Couldn't get mod versions for download: {err}");
                    self.set_popup_state(PopupState::NetworkError("Error getting versions for download", err));
                }
            }
            ModDownMsg::CategoriesReceived(res) => match res {
                Ok(val) => {
                    let des = &mut serde_json::Deserializer::from_slice(&val);
                    let result: Result<Vec<ModrinthCategory>, _> = serde_path_to_error::deserialize(des);
                    match result {
                        Ok(categories) => {
                            self.cached_categories = categories.into_iter().filter_map(|c|
                                if "mod" == c.project_type {
                                    Some(c.name)
                                } else {
                                    None
                                }
                            ).collect()
                        }
                        Err(err) => {
                            panic!("categories received error deserializing {err}")
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Couldn't get categories: {err}");
                    if !matches!(&self.popup_state, PopupState::NetworkError(_,_)) {
                        self.set_popup_state(PopupState::NetworkError("Error getting mod categories", err));
                    }

                }
            }
            ModDownMsg::ProviderButtonPressed(provider) => {
                self.current_provider = provider;
            }
            ModDownMsg::ModListingPressed(index) => {
                let id = self.mods_search_results[index].project_id.clone();
                if let Some(current_mod) = self.current_mod.as_mut() && id == current_mod.id {
                    'abort: {
                        if let Some(timestamp) = self.time_since_mod_button_clicked && timestamp.elapsed() <= Duration::from_millis(300) {
                            let Some(mod_data) = self.cached_mods.get_mut(&id) else { break 'abort };
                            if current_mod.is_in_selected_mod_list {
                                println!("IS IN SELECTED MOD LIST");
                                let i = self.selected_mod_versions.iter().find_position(|v| v.project_id == current_mod.id).unwrap().0;
                                self.selected_mod_versions.remove(i);
                                current_mod.is_in_selected_mod_list = false;
                                mod_data.is_in_selected_mod_list = false;
                            } else {
                                let Some(selected_version) = &current_mod.selected_version else { break 'abort };
                                current_mod.is_in_selected_mod_list = true;
                                mod_data.is_in_selected_mod_list = true;

                                self.selected_mod_versions.push(ModVersionQueued {
                                    icon_url: mod_data.icon_url.clone(),
                                    project_name: mod_data.title.clone(),
                                    project_id: mod_data.id.clone(),
                                    version_name: selected_version.name.clone(),
                                    version_id: selected_version.id.clone(),
                                    loaders: selected_version.loaders.clone(),
                                });
                            }
                            self.time_since_mod_button_clicked = None;
                            return Task::none();
                        }
                    }
                    self.time_since_mod_button_clicked = Some(Instant::now());
                    return Task::none();
                }

                let task = widget::operation::scroll_to(
                    widget::Id::new("markup"),
                scrollable::AbsoluteOffset::<f32>::default(),
                );
                if let Some(a_mod) = self.cached_mods.get(&id) {
                    self.current_mod = Some(ModInfo {
                        id: id.clone(),
                        is_in_selected_mod_list: a_mod.is_in_selected_mod_list,
                        ..Default::default()
                    });
                    return self.spotlight_mod();
                } else {
                    self.current_mod = Some(ModInfo {
                        id: id.clone(),
                        ..Default::default()
                    });
                    self.is_mod_fetching = FetchState::Fetching;
                    self.markup_state = MarkState::with_html("loading...");
                    return Task::batch([
                        Task::perform(reqwests::fetch_mod(id), ModDownMsg::ModReceived),
                        task,
                    ])
                    .map(|m| SuperMsg(m));
                }
            }
            ModDownMsg::ModReceived(res) => {
                match res {
                    Ok(val) => {
                        self.is_mod_fetching = FetchState::Done;
                        let des = &mut serde_json::Deserializer::from_slice(&val);
                        let result: Result<ModrinthMod, _> = serde_path_to_error::deserialize(des);
                        match result {
                            Ok(m) => {
                                println!("DIDDY AHH BLUD");
                                if let Some(a_mod) = &self.current_mod && a_mod.id == m.id {
                                    let id = m.id.clone();
                                    self.cached_mods.insert(m.id.clone(), m);
                                    return self.spotlight_mod();
                                }
                                self.cached_mods.insert(m.id.clone(), m);
                            }
                            Err(e) => {
                                let path = e.path().to_string();
                                eprintln!(
                                    "error parsing JSON from mod fetch: {} path: {path}\ndumping json: {}",
                                    e,
                                    String::from_utf8(val).unwrap_or_else(|e| e.to_string())
                                )
                            }
                        };
                    }
                    Err(err) => {
                        self.is_mod_fetching = FetchState::Errored;
                        eprintln!("Couldn't get json: {err}");
                        self.set_popup_state(PopupState::NetworkError("Error fetching mod", err));
                    }
                }
            },
            ModDownMsg::ModsListScrolled(viewport) => {
                if viewport.absolute_offset_reversed().y <= 300.0 {
                    if !self.scroll_load_debounce {
                        self.mods_search_offset += 20;
                        self.scroll_load_debounce = true;
                        return self._search_and_append();
                    }
                } // else {println!("debounce failed {}",test);}
                // } else {
                //     println!("turning off debounce");
                //     self.scroll_load_debounce = false
                // }
                // println!("scroll: {:?} offset_reversed: {:?}",viewport, viewport.absolute_offset_reversed());
            }
            ModDownMsg::SearchTyped(s) => {
                self.current_searchbar_text = s;
            }
            ModDownMsg::SearchSubmitted => {
                if self.current_searchbar_text == self.current_query {
                    return Task::none();
                }
                self.current_query = self.current_searchbar_text.clone();
                return self._new_mod_search();
            },
            ModDownMsg::RetrySearchPressed => {
                return self._new_mod_search();
            }
            ModDownMsg::FilterButtonPressed => self.show_filter_option = !self.show_filter_option,
            ModDownMsg::ShowSnapshotsChecked(b) => {
                self.show_snapshots = b;
            }
            ModDownMsg::ServerSideModsChecked(b) => {
                self.server_sided_mods_only = b;
                return self._new_mod_search();
            }
            ModDownMsg::FilterVersionPicked((v,s)) => {
                update_selection(&mut self.selected_filter_versions, v, s);
                return self._new_mod_search();
            }
            ModDownMsg::FilterLoaderPicked((l,s)) => {
                update_selection(&mut self.selected_filter_loaders, l, s);
                return self._new_mod_search();
            },
            ModDownMsg::FilterCategoryPicked((l,s)) => {
                update_selection(&mut self.selected_filter_categories, l, s);
                return self._new_mod_search();
            }
            ModDownMsg::ModVersionPicked(s) => {
                self.current_mod.as_mut().unwrap().selected_version = Some(s.clone());
            }
            ModDownMsg::SelectVersionButtonPressed => {
                let Some(current_mod) = self.current_mod.as_mut() else {return Task::none()};
                let mod_data = self.cached_mods.get_mut(&current_mod.id).unwrap();
                if current_mod.is_in_selected_mod_list {
                    println!("IS IN SELECTED MOD LIST");
                    let i = self.selected_mod_versions.iter().find_position(|v| v.project_id == current_mod.id).unwrap().0;
                    self.selected_mod_versions.remove(i);
                    current_mod.is_in_selected_mod_list = false;
                    mod_data.is_in_selected_mod_list = false;
                } else {
                    let Some(selected_version) = &current_mod.selected_version else { return Task::none() };
                    current_mod.is_in_selected_mod_list = true;
                    mod_data.is_in_selected_mod_list = true;

                    self.selected_mod_versions.push(ModVersionQueued {
                        icon_url: mod_data.icon_url.clone(),
                        project_name: mod_data.title.clone(),
                        project_id: mod_data.id.clone(),
                        version_name: selected_version.name.clone(),
                        version_id: selected_version.id.clone(),
                        loaders: selected_version.loaders.clone()
                    });
                }
            }
            ModDownMsg::SelectedVersionTrashPressed(i) => {
                let version = self.selected_mod_versions.remove(i);
                if let Some(current_mod) = &mut self.current_mod && current_mod.id == version.project_id {
                    current_mod.is_in_selected_mod_list = false;
                };
                self.cached_mods.get_mut(&version.project_id).unwrap().is_in_selected_mod_list = false;
            }
            ModDownMsg::DownloadButtonPressed => {
                let t = Task::perform(
                    reqwests::get_mod_versions(
                        self.selected_mod_versions.iter().map(|v| format!("\"{}\"",v.version_id)).collect()
                    ),
                    ModDownMsg::DownloadVersionsReceived
                ).map(|m| SuperMsg(m));
                self.set_popup_state(PopupState::DownloadConfirmation);
                return t;
            }

            ModDownMsg::ConfirmCloseButtonPressed => {
                return Task::done(Message::CloseWindow(WindowType::ModDownload));
            }
            ModDownMsg::CancelCloseButtonPressed => {
                self.popup_state = PopupState::None;
            }
        };
        Task::none()
    }

    pub fn view(&self) -> Element<'_, ModDownMsg> {
        let view = {
            let filter = if self.show_snapshots {
                |m: &&MinecraftVersion| {
                    match m.kind {
                        VersionKind::Release | VersionKind::Snapshot => true,
                        _ => false,
                    }
                }
            } else {
                |m: &&MinecraftVersion| {
                    if let VersionKind::Release = m.kind {true} else {false}
                }
            };
            let filter_options: Element<_> = if self.show_filter_option {
                column![
                    row![
                        pick_list_multi(MC_VERSIONS.get().unwrap().iter().filter(filter).cloned().collect_vec(), &self.selected_filter_versions, ModDownMsg::FilterVersionPicked)
                            .placeholder("showing all versions..")
                            .width(iced::Fill),
                        pick_list_multi(ModLoader::VARIANTS, &self.selected_filter_loaders, ModDownMsg::FilterLoaderPicked)
                            .placeholder("showing all loaders..")
                            .width(iced::Fill),
                        pick_list_multi(self.cached_categories.clone(), &self.selected_filter_categories, ModDownMsg::FilterCategoryPicked)
                            .placeholder("showing all categories..")
                            .width(iced::Fill)
                    ].spacing(5),
                    row![
                        checkbox(self.show_snapshots).label("List Snapshots⤴").on_toggle(ModDownMsg::ShowSnapshotsChecked).width(iced::Fill),
                        checkbox(self.server_sided_mods_only).label("Only show server-sided mods").on_toggle(ModDownMsg::ServerSideModsChecked).width(iced::Fill)
                    ],
                    space().height(2),
                    rule::horizontal(1)
                ].spacing(5).into()
            } else {
                space().into()
            };

            let provider_button = |icon: SvgHandle, color: Color, name: &'static str, provider: ModProvider| -> Button<ModDownMsg> {
                let b = button(container(row![
                    svg(icon).style(move |t, _| svg::Style {
                        color: Some(color)
                    }).width(20).height(20),
                    text(name)
                ].spacing(5)).center_x(iced::Fill).align_y(iced::Center).padding(2));
                if provider == self.current_provider {
                    b
                } else {
                    b.on_press(ModDownMsg::ProviderButtonPressed(provider))
                }
            };

            column![
                row![
                    provider_button(STATIC_IMAGES.modrinth.clone(), Color::from_rgb8(27, 217, 106), "Modrinth", ModProvider::Modrinth),
                    provider_button(STATIC_IMAGES.curseforge.clone(), Color::from_rgb8(255, 120, 77), "Curseforge", ModProvider::Curseforge),
                ].spacing(5),
                row![
                    text_input("Search...", &self.current_searchbar_text).on_input(|s| ModDownMsg::SearchTyped(s)).on_submit(ModDownMsg::SearchSubmitted).icon(text_input::Icon { font: iced::Font::DEFAULT, code_point: '⌕', size: None, spacing: 4.0, side: text_input::Side::Left }),
                    space().width(5),
                    button(svg(STATIC_IMAGES.filter.clone())).width(30).height(30).padding(4).on_press(ModDownMsg::FilterButtonPressed).style(button::secondary)
                ].align_y(iced::Center),
                filter_options,
                row![
                    scrollable(// mod list
                        if self.mods_search_results.is_empty() {
                            match self.is_search_fetching {
                                FetchState::Fetching => Element::new(container(circular::Circular::new()).center(100)),
                                FetchState::Done => center("no mods found :(").padding(20).into(),
                                FetchState::Errored => column![
                                    "error searching mods :(",
                                    button("retry").on_press(ModDownMsg::RetrySearchPressed).style(|t,s|button::secondary(t,s))
                                ].spacing(10).align_x(iced::Center).width(iced::Fill).padding(20).into()
                            }
                        } else {
                            column((0..self.mods_search_results.len()).into_iter().map(|i| self._create_mod_listing(i))).spacing(5).into()
                        }
                    ).width(320).height(iced::Fill).spacing(5).on_scroll(ModDownMsg::ModsListScrolled).id(widget::Id::new("search")),
                    column![scrollable( // markdown section
                        if let Some(a_mod) = &self.current_mod && let Some(listing) = self.cached_mods.get(&a_mod.id) {
                            const IMG_SIZE:u32 = 100;
                            let thumbnail: Element<ModDownMsg> = if let Some(url) = &listing.icon_url {
                                if let Some(img) = self.cached_images.get(url).cloned() {
                                    match img {
                                        ImageType::Svg(handle) => svg(handle).width(IMG_SIZE).height(IMG_SIZE).into(),
                                        ImageType::Raster(handle) => image(handle).width(IMG_SIZE).height(IMG_SIZE).into(),
                                    }
                                } else {
                                    eprintln!("Image not found in cache for search builder! Creating default image");
                                    image(&STATIC_IMAGES.missing).width(IMG_SIZE).height(IMG_SIZE).into()
                                }
                            } else {
                                eprintln!("nonexistent thumbnail in search builder");
                                image(&STATIC_IMAGES.missing).width(IMG_SIZE).height(IMG_SIZE).into()
                            };
                            let bg_color = match listing.color {
                                Some(c) => Color {
                                    r: c.r.clamp(0., 0.5),
                                    g: c.g.clamp(0., 0.5),
                                    b: c.b.clamp(0., 0.5),
                                    a: 1.0
                                },
                                None => Color::from_rgb8(50, 50, 60)
                            };
                            column![
                                container(
                                    row![thumbnail,column![
                                        rich_text![span(&listing.title).link(format!("https://modrinth.com/mod/{}",listing.slug)).color(Color::from_rgb8(175, 200, 240))]
                                            .size(32).line_height(text::LineHeight::Relative(1.0)).on_link_click(ModDownMsg::OpenLink),
                                        text(&listing.description).color(Color::WHITE).line_height(text::LineHeight::Relative(1.2))
                                    ].spacing(2)].spacing(10)).style(move |_: &_| {
                                        let gradient = Linear::new(Radians(PI/2.0))
                                            .add_stop(0.0, bg_color)
                                            .add_stop(1.0, Color::TRANSPARENT)
                                            .into();

                                        container::Style {
                                            background: Some(iced::Background::Gradient(gradient)),
                                            ..Default::default()
                                        }
                                    }),
                                // row![
                                //     svg(STATIC_IMAGES.download.clone()).width(14).height(14).style(|t: &iced::Theme, _|svg::Style {color: Some(Color::from_rgb8(150,150,150))}),
                                //     bold(listing.downloads.abbreviate_number(&Default::default())).style(|t: &iced::Theme|text::Style {color: Some(Color::from_rgb8(150,150,150))}).size(12),
                                //     bold(listing.game_versions.join(",")).style(|t: &iced::Theme|text::Style {color: Some(Color::from_rgb8(150,150,150))}).size(12)
                                // ],
                                // rule::horizontal(1),
                                MarkWidget::new(&self.markup_state).on_clicking_link(|url| {ModDownMsg::OpenLink(url)})
                                    .on_drawing_image(|info| {
                                        if let Some(img) = self.cached_images.get(info.url).cloned() {
                                            match img {
                                                ImageType::Svg(handle) => {
                                                    let mut img = svg(handle);
                                                    if let Some(w) = info.width {
                                                        img = img.width(w)
                                                    }
                                                    if let Some(h) = info.height {
                                                        img = img.height(h);
                                                    }
                                                    img.into()
                                                }
                                                ImageType::Raster(handle) => {
                                                    let mut img = image(handle);
                                                    if let Some(w) = info.width {
                                                        img = img.width(w)
                                                    }
                                                    if let Some(h) = info.height {
                                                        img = img.height(h);
                                                    }
                                                    img.into()
                                                }
                                            }
                                        } else {
                                            //eprintln!("missing image in markdown builder"); //bluid is sussy and amond us idk if we can make it out alive guys. I have a paln to capiuture imporster and bring ande end to among us. fhe the ifn sutss6y and the amkjgh us` 89is tsupper green and the doced is green like catctus pvz i like hgarden warefare so kuch I bought games and dlc on garden warefare`
                                            image(&STATIC_IMAGES.missing).width(128).height(128).into()
                                        }
                                    }
                                )
                            ].spacing(8).into()
                        } else {
                            match self.is_mod_fetching {
                                FetchState::Fetching => center(circular::Circular::new()).into(),
                                FetchState::Done => Element::new(space()),
                                FetchState::Errored => center("error fetching mod :(").into()
                            }
                        }
                    ).height(iced::Fill).width(iced::Fill).spacing(5).id(widget::Id::new("markup")), {
                        let (select_button, picker): (Button<ModDownMsg>,Element<ModDownMsg>) = if let Some(current_mod) = &self.current_mod {
                            if current_mod.is_in_selected_mod_list {
                                let selected_version = self.selected_mod_versions.iter().find(|x| x.project_id == current_mod.id).expect(&format!("{} says is_in_selected_mod_list but isn't actually in selected mod list!!!",current_mod.id));

                                (button(text("Deselect").center()).on_press(ModDownMsg::SelectVersionButtonPressed).width(80),
                                stack![icon_pick_list(
                                    [selected_version.clone()], Some(selected_version.clone()), |_| ModDownMsg::None,
                                        |v| v.loaders.iter().map(|l| match l {
                                            ModLoader::Fabric => SVG_MOD_LOADERS[1].clone(),
                                            ModLoader::NeoForge => SVG_MOD_LOADERS[2].clone(),
                                            ModLoader::Forge => SVG_MOD_LOADERS[3].clone(),
                                            ModLoader::Paper => SVG_MOD_LOADERS[4].clone(),
                                            ModLoader::Purpur => SVG_MOD_LOADERS[5].clone(),
                                            ModLoader::Folia => SVG_MOD_LOADERS[6].clone(),
                                            ModLoader::Velocity => SVG_MOD_LOADERS[7].clone(),
                                            _ => SVG_MOD_LOADERS[0].clone(),
                                        }).collect_vec())
                                        .width(iced::Fill)
                                        .style(|t: &iced::Theme, _| icon_pick_list::Style {
                                            background: t.extended_palette().background.weaker.color.into(),
                                            border: border::color(t.extended_palette().secondary.base.color),
                                            text_color: Color::from_rgb8(150, 150, 150).into(),
                                            ..icon_pick_list::default(t, icon_pick_list::Status::Active)
                                        }),
                                    opaque(space().width(iced::Fill).height(iced::Fill))
                                ].into())
                            } else if !current_mod.cached_versions.is_empty() {
                                    (button(text("Select").center()).on_press(ModDownMsg::SelectVersionButtonPressed).width(80),
                                    icon_pick_list(current_mod.cached_versions.clone(), current_mod.selected_version.clone(), ModDownMsg::ModVersionPicked,
                                        |v:&ModVersion| v.loaders.iter().map(|l| match l {
                                            ModLoader::Fabric => SVG_MOD_LOADERS[1].clone(),
                                            ModLoader::NeoForge => SVG_MOD_LOADERS[2].clone(),
                                            ModLoader::Forge => SVG_MOD_LOADERS[3].clone(),
                                            ModLoader::Paper => SVG_MOD_LOADERS[4].clone(),
                                            ModLoader::Purpur => SVG_MOD_LOADERS[5].clone(),
                                            ModLoader::Folia => SVG_MOD_LOADERS[6].clone(),
                                            ModLoader::Velocity => SVG_MOD_LOADERS[7].clone(),
                                            _ => SVG_MOD_LOADERS[0].clone(),
                                        }).collect_vec())
                                        .width(iced::Fill).into())

                            } else {
                                (button(text("Select").center()).width(80),
                                container(text(""))
                                    .padding([5, 10])
                                    .width(iced::Fill)
                                    .style(|t: &iced::Theme| container::Style {
                                            background: Some(t.extended_palette().background.weaker.color.into()),
                                            border: border::rounded(4),
                                            ..Default::default()
                                        })
                                    .into())

                            }
                        } else {
                            (button(text("Select").center()).width(80),
                            container(text(""))
                                .padding([5, 10])
                                .width(iced::Fill)
                                .style(|t: &iced::Theme| container::Style {
                                        background: Some(t.extended_palette().background.weaker.color.into()),
                                        border: border::rounded(4),
                                        ..Default::default()
                                    })
                                .into())

                        };

                        row![
                            picker,
                            select_button
                        ].spacing(5)
                    }].spacing(10)
                ].spacing(10),

                row![
                    scrollable( container(
                        row(self.selected_mod_versions.iter().enumerate().map(|(i, v)| {
                            const IMG_SIZE: u32 = 48;
                            let img:Element<_> = if let Some(url) = &v.icon_url && let Some(img) = self.cached_images.get(url) {
                                match img {
                                    ImageType::Svg(handle) => {
                                        svg(handle.clone()).width(IMG_SIZE).height(IMG_SIZE).into()
                                    }
                                    ImageType::Raster(handle) => {
                                        image(handle).width(IMG_SIZE).height(IMG_SIZE).into()
                                    }
                                }
                            } else {
                                image(&STATIC_IMAGES.missing).width(IMG_SIZE).height(IMG_SIZE).into()
                            };
                            tooltip(
                                mouse_area(
                                    hover(
                                        img,
                                        container(svg(STATIC_IMAGES.trashcan.clone()).width(32).height(32)).center(iced::Fill)
                                            .style(|_| container::Style {
                                                background: Some(Color::from_rgba(0.1,0.1,0.1,0.4).into()),
                                                ..Default::default()
                                        })
                                    )
                                ).on_press(ModDownMsg::SelectedVersionTrashPressed(i)),
                                container(column![
                                    bold(&v.project_name).size(14).color(Color::WHITE),
                                    text(&v.version_name).size(10),
                                    row(v.loaders.iter().map(|l|svg(match l {
                                        ModLoader::Fabric => STATIC_IMAGES.fabric.clone(),
                                        ModLoader::NeoForge => STATIC_IMAGES.neoforge.clone(),
                                        ModLoader::Forge => STATIC_IMAGES.forge.clone(),
                                        ModLoader::Paper => STATIC_IMAGES.paper.clone(),
                                        ModLoader::Purpur => STATIC_IMAGES.purpur.clone(),
                                        ModLoader::Folia => STATIC_IMAGES.folia.clone(),
                                        ModLoader::Velocity => STATIC_IMAGES.velocity.clone()
                                    }).width(20).height(20).into())).spacing(5)
                                ].align_x(iced::Center).padding(8).spacing(5)).style(|t:&iced::Theme|container::Style {
                                    background: Some(t.extended_palette().background.weakest.color.into()),
                                    border: Border::default().rounded(10),
                                    ..Default::default()
                                }),
                                tooltip::Position::Top
                            ).gap(8).into()
                        })).padding(4).spacing(4)
                    )).height(52).width(400).anchor_y(scrollable::Anchor::End)
                        .direction(scrollable::Direction::Horizontal(Scrollbar::new().width(0).scroller_width(6)))
                        .style(|theme: &iced::Theme, status| {
                            let empty_rail = scrollable::Rail {
                                background: None,
                                border: Border::default(),
                                scroller: scrollable::Scroller {
                                    background: iced::Background::Color(Color::TRANSPARENT),
                                    border: Border::default(),
                                },
                            };
                            let mut style = scrollable::Style {
                                container: theme.extended_palette().background.weaker.color.into(),
                                ..scrollable::default(theme, status)
                            };
                            if let scrollable::Status::Active {
                                is_horizontal_scrollbar_disabled,
                                is_vertical_scrollbar_disabled,
                                ..
                            } = status
                            {
                                if !is_horizontal_scrollbar_disabled {
                                    style.horizontal_rail = empty_rail;
                                }
                                if !is_vertical_scrollbar_disabled {
                                    style.vertical_rail = empty_rail;
                                }
                            }
                            style
                    }
                    ),
                    button("Download").on_press(ModDownMsg::DownloadButtonPressed)
                ]
            ].spacing(10).padding(15)
        };

        match &self.popup_state {
            PopupState::None => stack![view].into(),
            PopupState::CloseConfirmation => stack![
                view,
                Self::create_dialog(
                    "Confirm close",
                    format!("Are you sure you want to close?\nyou have {} mods selected.",self.selected_mod_versions.len()),
                    row![button("Close").on_press(ModDownMsg::ConfirmCloseButtonPressed),
                    button("Cancel").on_press(ModDownMsg::CancelCloseButtonPressed).style(|t: &iced::Theme,s| {
                        button::secondary(t,s)
                    })].spacing(5).into()
                )
            ].into(),
            PopupState::NetworkError(title, body) => stack![
                view,
                Self::create_dialog(
                    title,
                    body.clone(),
                    button("Ok :(").on_press(ModDownMsg::CancelCloseButtonPressed).into()
                )
            ].into(),
            PopupState::DownloadConfirmation => stack![
                view,
                opaque(center(container(column![
                    bold("Confirm Download").size(20),
                    text("are you sure you want to download:"),
                    row![space().width(15),iced_selection::text("are you sure?").line_height(1.5).wrapping(text::Wrapping::WordOrGlyph)],
                    right(row![
                        button("Cancel").on_press(ModDownMsg::CancelCloseButtonPressed).style(|t: &iced::Theme,s| {
                            button::secondary(t,s)
                        })
                    ])
                ].padding(25).spacing(8)
                ).width(600).height(650)
                    .style(|t: &iced::Theme| container::Style {
                        background: Some(deviate(t.palette().background,-0.0).into()),
                        border: Border::default().rounded(10).color(t.extended_palette().background.stronger.color).width(1.5),
                        ..Default::default()
                    })
                ).center(iced::Fill))
            ].into()
        }
    }

    pub fn new(program_data: &crate::ProgramData) -> (Self, Task<Message>) {
        let mut state = ModDownloaderState::default();
        state.server_sided_mods_only = true;

        update_selection(&mut state.selected_filter_versions, Some(program_data.version.clone()), SelectionState::Included);
        update_selection(&mut state.selected_filter_loaders, Some(program_data.loader), SelectionState::Included);
        let task = Task::batch([
            state._new_mod_search(),
            Task::perform(reqwests::get_categories(), ModDownMsg::CategoriesReceived).map(|m|SuperMsg(m))
        ]);
        (state,task)
    }

    fn set_popup_state(&mut self, state: PopupState) {
        self.popup_state = match state {
            PopupState::None => {PopupState::None}
            PopupState::CloseConfirmation => {PopupState::CloseConfirmation},
            PopupState::NetworkError(title, body) => {
                if let PopupState::CloseConfirmation = self.popup_state {return}
                if let PopupState::NetworkError(_, _) = self.popup_state {return}
                PopupState::NetworkError(title, body)
            }
            PopupState::DownloadConfirmation => {
                if let PopupState::CloseConfirmation = self.popup_state {return}
                if let PopupState::NetworkError(_, _) = self.popup_state {return}
                PopupState::DownloadConfirmation
            }
        }
    }

    fn spotlight_mod(&mut self) -> Task<Message> {
        let Some(mod_info) = &self.current_mod else {return Task::none()};
        let mod_data = self.cached_mods.get(&mod_info.id).unwrap();
        self.markup_state = MarkState::with_html_and_markdown(&mod_data.body);

        println!("{:#?}",self.current_mod);

        Task::batch([
            self._get_mod_versions(mod_info.id.clone()),
            self._download_markup_images().map(|m| SuperMsg(m)),
        ])
    }
    fn create_dialog<'a>(title: &'static str, body: String, buttons: Element<'a, ModDownMsg>) -> Element<'a, ModDownMsg> {
        opaque(center(container(column![
            iced_selection::text(title).size(20).font(Font {weight: font::Weight::Bold, ..Default::default()}),
            row![space().width(15),iced_selection::text(body).line_height(1.5).wrapping(text::Wrapping::WordOrGlyph)],
            right(buttons)
        ].padding(25).spacing(8)
        ).width(600)
            .style(|t: &iced::Theme| container::Style {
                background: Some(deviate(t.palette().background,-0.0).into()),
                border: Border::default().rounded(10).color(t.extended_palette().background.stronger.color).width(1.5),
                ..Default::default()
            })
        ).center(iced::Fill))

    }

    fn _create_mod_listing(&'_ self, mods_list_index: usize) -> Element<'_, ModDownMsg> {
        const IMG_SIZE: u32 = 75;
        let listing = &self.mods_search_results[mods_list_index];
        let is_selected = self.cached_mods.get(&listing.project_id).map_or(false, |m|m.is_in_selected_mod_list);
        let thumbnail: Element<ModDownMsg> = if let Some(url) = &listing.icon_url {
            if let Some(img) = self.cached_images.get(url).cloned() {
                match img {
                    ImageType::Svg(handle) => {
                        widget::svg(handle).width(IMG_SIZE).height(IMG_SIZE).into()
                    }
                    ImageType::Raster(handle) => widget::image(handle)
                        .width(IMG_SIZE)
                        .height(IMG_SIZE)
                        .into(),
                }
            } else {
                // eprintln!("Image not found in cache for search builder! Creating default image");
                widget::image(&STATIC_IMAGES.missing)
                    .width(IMG_SIZE)
                    .height(IMG_SIZE)
                    .into()
            }
        } else {
            eprintln!("nonexistent thumbnail in search builder");
            widget::Image::new(&STATIC_IMAGES.missing)
                .width(IMG_SIZE)
                .height(IMG_SIZE)
                .into()
        };
        let title = if is_selected {
            bold(&listing.title)
        } else {
            text(&listing.title)
        };
        let contents = row![
            container(thumbnail).padding([0, 5]),
            column![
                title,
                text(&listing.description).size(10)
            ]
            .max_width(215)
        ];
        let button = button(contents)
            .on_press_with(move || ModDownMsg::ModListingPressed(mods_list_index))
            .height(80)
            .width(iced::Fill)
            .padding([5, 0])
            .style(move |t,s| {
                let pair = if let Some(a_mod) = &self.current_mod && a_mod.id == listing.project_id {
                    t.extended_palette().secondary.weak
                } else {
                    t.extended_palette().primary.base
                };
                button::Style {
                    background: Some(pair.color.into()),
                    border: if is_selected {
                        border::rounded(2).width(4).color(deviate(pair.color.into(), 0.1))
                    } else {
                        border::rounded(2)
                    },
                    text_color: pair.text,
                    ..Default::default()
                }
            });
        button.into()
    }

    fn _download_markup_images(&mut self) -> Task<ModDownMsg> {
        Task::batch(self.markup_state.find_image_links().into_iter().filter_map(|url| {
            if self.images_queued.insert(url.clone()) {
                Some(Task::perform(reqwests::download_image(url), ModDownMsg::ImageDownloaded))
            } else {
                None
            }
        }))
    }

    fn _new_mod_search(&mut self) -> Task<Message> {
        self.is_search_fetching = FetchState::Fetching;
        self.search_fetching_sequence_number += 1;
        self.mods_search_results.clear();
        self.mods_search_offset = 0;

        self.current_mod = None;
        Task::batch([
                        self._search_and_append(),
            widget::operation::scroll_to(
                widget::Id::new("search"),
                scrollable::AbsoluteOffset::<f32>::default(),
            ),
        ])
    }

    fn _search_and_append(&mut self) -> Task<Message> {
        let mut args: Vec<String> = Vec::new();
        if self.server_sided_mods_only { args.push("\"server_side!=unsupported\"".to_string())}
        if !self.selected_filter_versions.is_empty() {
            let a = self.selected_filter_versions.iter().map(|(v,_)|format!("\"versions:{}\"",v.as_ref().unwrap().id)).join(",");
            args.push(a);
        }
        if !self.selected_filter_loaders.is_empty() {
            let a = self.selected_filter_loaders.iter().map(|(l,_)|format!("\"categories:{}\"",l.unwrap().to_string())).join(",");
            args.push(a);
        }
        if !self.selected_filter_categories.is_empty() {
            let a = self.selected_filter_categories.iter().map(|(c,_)|format!("\"categories:{}\"",c.as_ref().unwrap()));
            args.extend(a);
        }

        Task::perform(
            reqwests::search_mods(
                self.mods_search_offset,
                Some(self.current_query.clone()),
                args,
            ),
            ModDownMsg::ModsSearchReceived,
        ).map(|m|SuperMsg(m))
    }

    fn _get_mod_versions(&mut self, id:String) -> Task<Message> {
        let loaders = self.selected_filter_loaders.iter().map(|(l,_)|l.unwrap().to_string().to_ascii_lowercase()).collect();
        let game_versions = self.selected_filter_versions.iter().map(|(v,_)|v.as_ref().unwrap().id.clone()).collect();

        Task::perform(reqwests::get_available_mod_versions(id, loaders, game_versions), ModDownMsg::ModVersionsReceived).map(|m|SuperMsg(m))
    }
}

#[derive(Default, Debug)]
struct ModInfo {
    id: String,
    cached_versions: Vec<ModVersion>,
    selected_version: Option<ModVersion>,
    is_in_selected_mod_list: bool,
}

#[derive(Debug, serde::Deserialize)]
struct ModrinthMod {
    #[serde(skip_deserializing)]
    is_in_selected_mod_list: bool,

    slug: String,
    title: String,
    id: String,
    body: String,
    description: String,
    versions: Vec<String>,
    game_versions: Vec<String>,
    loaders: Vec<String>,
    categories: Vec<String>,
    additional_categories: Vec<String>,
    issues_url: Option<String>,
    source_url: Option<String>,
    wiki_url: Option<String>,
    discord_url: Option<String>,
    client_side: String,
    server_side: String,
    project_type: String,
    downloads: i64,
    icon_url: Option<String>,
    #[serde(deserialize_with = "mmod_color_handler")]
    color: Option<Color>,
    followers: i64,
    published: String,
    updated: String,
    // gallery: Vec<String>,
}

struct SearchListing {
    title: String,
    description: String,
    icon_url: Option<String>,
    project_id: String,
    author: String,
    downloads: i64,
}

#[derive(Debug, serde::Deserialize)]
struct ModrinthSearchResult {
    slug: String,
    title: String,
    project_id: String,
    author: String,
    description: String,
    versions: Vec<String>,
    categories: Vec<String>,
    client_side: String,
    server_side: String,
    project_type: String,
    downloads: i64,
    icon_url: Option<String>,
    #[serde(deserialize_with = "mmod_color_handler", default)]
    color: Option<Color>,
    follows: i64,
    date_created: String,
    date_modified: String,
    latest_version: String,
    license: String,
    gallery: Vec<String>,
    featured_gallery: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct ModrinthSearch {
    hits: Vec<ModrinthSearchResult>,
    offset: i64,
    limit: i64,
    total_hits: i64,
}

#[derive(Debug, serde::Deserialize)]
struct ModrinthVersionDownload { // used for the list of versions (and dependencies) when "download" is pressed.
    name: String,
    id: String,
    project_id: String,

    loaders: Vec<String>,

    version_number: String,
    changelog: String,
    dependencies: Vec<ModrinthDependency>,
    version_type: String,
}

#[derive(Debug, serde::Deserialize)]
struct ModrinthVersionMass { // used for the list of versions in a mod listing
    name: String,
    id: String,
    loaders: Vec<String>,
}

#[derive(Clone, Debug)]
struct ModVersion {
    name: String,
    id: String,

    loaders: Vec<ModLoader>,
}
impl Display for ModVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}",self.name)
    }
}
impl PartialEq for ModVersion {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

struct ModrinthFile {

}

#[derive(Debug)]
struct ModVersionQueued { // it's a separate struct so it can have a nice icon and stuff
    icon_url: Option<String>,

    project_name: String,
    project_id: String,
    version_name: String,
    version_id: String,

    loaders: Vec<ModLoader>,
}
impl Display for ModVersionQueued {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.version_name)
    }
}
impl PartialEq for ModVersionQueued {
    fn eq(&self, other: &Self) -> bool {
        self.version_id == other.version_id
    }
}

#[derive(Debug, serde::Deserialize)]
struct ModrinthDependency {
    version_id: Option<String>,
    project_id: Option<String>,
    file_name: Option<String>,
    dependency_type: String,
}

fn mmod_color_handler<'de, D>(deserializer: D) -> Result<Option<Color>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let Some(v): Option<i64> = Deserialize::deserialize(deserializer)? else {
        return Ok(None);
    };

    let b = (v % 256).try_into().map_err(serde::de::Error::custom)?;
    let g = ((v >> 8) % 256)
        .try_into()
        .map_err(serde::de::Error::custom)?;
    let r = ((v >> 16) % 256)
        .try_into()
        .map_err(serde::de::Error::custom)?;

    Ok(Some(Color::from_rgb8(r, g, b)))
}

#[derive(Debug, serde::Deserialize)]
struct ModrinthCategory {
    name: String,
    project_type: String,
}
