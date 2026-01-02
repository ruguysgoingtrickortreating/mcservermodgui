use std::{
    collections::{HashMap, HashSet},
    f32::consts::PI,
};
use std::fmt::Display;
use std::time::Instant;
use frostmark::{MarkState, MarkWidget};
use iced::{Color, Element, Radians, Task, advanced::{image::Handle as RasterHandle, svg::Handle as SvgHandle}, gradient::Linear, widget::{
    self, button, column, container, row, scrollable, checkbox, image, mouse_area, space, stack, svg, Button,
    scrollable::Viewport,
    text, text_input,
}, border, mouse, Border, Font, font};
use iced::widget::scrollable::Scrollbar;
use iced::widget::tooltip;
use iced_widget_extra::pick_list_multi;
use iced_widget_extra::pick_list_multi::{update_selection, SelectionState};
use itertools::Itertools;
use serde::Deserialize;
use smart_default::{self, SmartDefault};
use strum::VariantArray;
use crate::{ImageType, STATIC_IMAGES, Message::{self, ModDLMessage as SuperMsg}, reqwests, MinecraftVersion, MC_VERSIONS, ProgramData, ModLoader, SVG_MOD_LOADERS};
use crate::util::icon_pick_list;
use crate::util::icon_pick_list::{icon_pick_list, Catalog};

#[derive(Debug, Clone)]
pub enum ModDownMsg {
    OpenLink(String),

    ImageDownloaded(Result<reqwests::ImageData, String>),
    ModsSearchReceived(Result<Vec<u8>, String>),
    ModReceived(Result<Vec<u8>, String>),
    ModVersionsReceived(Result<(String, Vec<u8>), String>),
    ModListingPressed(usize),
    ModsListScrolled(Viewport),
    SearchTyped(String),
    SearchSubmitted,
    FilterButtonPressed,
    FilterVersionPicked((Option<MinecraftVersion>, SelectionState)),
    FilterLoaderPicked((Option<ModLoader>, SelectionState)),
    ServerSideModsChecked(bool),
    DownloadButtonPressed,
    ModVersionPicked(ModVersion),
    SelectVersionButtonPressed,

    None
}

#[derive(SmartDefault)]
pub struct ModDownloaderState {
    markup_state: MarkState,

    current_mod: Option<ModInfo>,
    cached_images: HashMap<String, ImageType>,
    cached_mods: HashMap<String, ModrinthMod>,
    images_queued: HashSet<String>,

    selected_mod_versions: Vec<ModVersionQueued>,

    current_searchbar_text: String,
    current_query: String,
    mods_search_results: Vec<ModrinthSearchResult>,
    mods_search_offset: u64,

    show_filter_option: bool,
    server_sided_mods_only: bool,
    selected_filter_versions: Vec<(Option<MinecraftVersion>, SelectionState)>,
    selected_filter_loaders: Vec<(Option<ModLoader>,SelectionState)>,

    // selected_mod_version: ModrinthVersion,

    #[default(_code = "Instant::now()")]
    time_since_mod_button_clicked: Instant,
    scroll_load_debounce: bool,
    is_search_fetching: bool,
    is_mod_fetching: bool,
}
impl ModDownloaderState {
    pub fn update(&mut self, _message: ModDownMsg) -> Task<Message> {
        match _message {
            ModDownMsg::OpenLink(url) => return Task::done(Message::OpenLink(url)),
            ModDownMsg::None => (),

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
                    self.is_search_fetching = false;
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
                    eprintln!("Couldn't get json: {err}");
                }
            },
            ModDownMsg::ModVersionsReceived(res) => match res {
                Ok((id, val)) => {
                    let Some( current_mod) = self.current_mod.as_mut() else {
                        return Task::none()
                    };
                    if id != current_mod.id {return Task::none()}

                    let des = &mut serde_json::Deserializer::from_slice(&val);
                    let result: Result<Vec<ModrinthVersion>, _> = serde_path_to_error::deserialize(des);
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
                            current_mod.cached_versions = dbg!(v);
                        }
                        Err(err) => {
                            panic!("versions recieved error deserializing {err}")
                        }
                    }
                }
                Err(err) => {panic!("{err}")}
            }
            ModDownMsg::ModListingPressed(index) => {
                let id = self.mods_search_results[index].project_id.clone();
                if let Some(current_mod) = self.current_mod.as_mut() && id == current_mod.id {
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
                    self.is_mod_fetching = true;
                    self.markup_state = MarkState::with_html("loading...");
                    return Task::batch([
                        Task::perform(reqwests::fetch_mod(id), ModDownMsg::ModReceived),
                        task,
                    ])
                    .map(|m| SuperMsg(m));
                }
            }
            ModDownMsg::ModReceived(res) => {
                self.is_mod_fetching = false;
                match res {
                    Ok(val) => {
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
                        eprintln!("Couldn't get json: {err}");
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
            ModDownMsg::FilterButtonPressed => self.show_filter_option = !self.show_filter_option,
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
            ModDownMsg::ModVersionPicked(s) => {
                self.current_mod.as_mut().unwrap().selected_version = Some(s.clone());
            }
            ModDownMsg::SelectVersionButtonPressed => {
                let Some(current_mod) = self.current_mod.as_mut() else {return Task::none()};
                let mod_data = self.cached_mods.get_mut(&current_mod.id).unwrap();
                if current_mod.is_in_selected_mod_list {
                    println!("IS IN SELECTED MOD LIST");
                    let i = self.selected_mod_versions.iter().find_position(|v| v.project_id == current_mod.id).unwrap();
                    self.selected_mod_versions.remove(i.0);
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
            ModDownMsg::DownloadButtonPressed => {
                let s = self.cached_mods.get(&self.current_mod.as_ref().unwrap().id).unwrap();

                dbg!(&s);
            }
        };
        Task::none()
    }

    pub fn view(&self) -> Element<'_, ModDownMsg> {
        let filter_options: Element<_> = if self.show_filter_option {
            row![
                pick_list_multi(ModLoader::VARIANTS, &self.selected_filter_loaders, ModDownMsg::FilterLoaderPicked).placeholder("Select Version")
                    .width(iced::Fill)
                    .none_label("Pick a version"),
                pick_list_multi(MC_VERSIONS.get().unwrap().clone(), &self.selected_filter_versions, ModDownMsg::FilterVersionPicked).placeholder("Select Version")
                    .width(iced::Fill)
                    .none_label("Pick a version"),
                checkbox(self.server_sided_mods_only).label("Only show server-sided mods").on_toggle(ModDownMsg::ServerSideModsChecked)
            ].into()
        } else {
            space().into()
        };

        column![
            row![
                text_input("Search...", &self.current_searchbar_text).on_input(|s| ModDownMsg::SearchTyped(s)).on_submit(ModDownMsg::SearchSubmitted).icon(text_input::Icon { font: iced::Font::DEFAULT, code_point: '⌕', size: None, spacing: 4.0, side: text_input::Side::Left }),
                space().width(5),
                button(svg(STATIC_IMAGES.filter.clone())).width(30).height(30).padding(4).on_press(ModDownMsg::FilterButtonPressed).style(button::secondary)
            ].align_y(iced::Center),
            filter_options,
            row![
                scrollable(// mod list
                    if self.mods_search_results.is_empty() {
                        if self.is_search_fetching {
                            iced::Element::from(container("loading...").center(100))
                        } else {
                            container("no mods found :(").center(100).into()
                        }
                    } else {
                        column((0..self.mods_search_results.len()).into_iter().map(|i| self._create_mod_listing(i))).spacing(5).into()
                    }
                ).width(320).spacing(5).on_scroll(ModDownMsg::ModsListScrolled).id(widget::Id::new("search")),
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
                                    text(&listing.title).size(32).line_height(text::LineHeight::Relative(1.0)),
                                    text(&listing.description).color(Color::from_rgb8(200, 200, 200)).line_height(text::LineHeight::Relative(1.2))
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
                                        eprintln!("missing image in markdown builder"); //bluid is sussy and amond us idk if we can make it out alive guys. I have a paln to capiuture imporster and bring ande end to among us. fhe the ifn sutss6y and the amkjgh us` 89is tsupper green and the doced is green like catctus pvz i like hgarden warefare so kuch I bought games and dlc on garden warefare`
                                        image(&STATIC_IMAGES.missing).width(128).height(128).into()
                                    }
                                }
                            )
                        ].spacing(10).into()
                    } else {Element::from("")}
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
                                mouse_area(space().height(iced::Fill).width(iced::Fill)).on_press(ModDownMsg::None).interaction(mouse::Interaction::Idle)
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
                    row(self.selected_mod_versions.iter().map(|v| {
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
                            img,
                            container(column![
                                text(&v.project_name).font(Font {weight: font::Weight::Bold, ..Default::default()}).size(14).color(Color::WHITE),
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
                            ].align_x(iced::Center).padding(5).spacing(5)).style(|t:&iced::Theme|container::Style {
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
        ].spacing(10).padding(15).into()
    }

    pub fn new(program_data: &crate::ProgramData) -> (Self, Task<Message>) {
        let mut state = ModDownloaderState::default();
        state.server_sided_mods_only = true;

        update_selection(&mut state.selected_filter_versions, Some(program_data.version.clone()), SelectionState::Included);
        update_selection(&mut state.selected_filter_loaders, Some(program_data.loader), SelectionState::Included);
        let task = state._new_mod_search();
        (state,task)
    }

    pub fn spotlight_mod(&mut self) -> Task<Message> {
        let Some(mod_info) = &self.current_mod else {return Task::none()};
        let mod_data = self.cached_mods.get(&mod_info.id).unwrap();
        self.markup_state = MarkState::with_html_and_markdown(&mod_data.body);

        println!("{:#?}",self.current_mod);

        Task::batch([
            self._get_mod_versions(mod_info.id.clone()),
            self._download_markup_images().map(|m| SuperMsg(m)),
        ])
    }

    fn _create_mod_listing(&'_ self, mods_list_index: usize) -> Element<'_, ModDownMsg> {
        const IMG_SIZE: u32 = 75;
        let listing = &self.mods_search_results[mods_list_index];
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
        let contents = row![
            container(thumbnail).padding([0, 5]),
            column![
                listing.title.as_str(),
                text(&listing.description)
                    .size(10)
                    .color(Color::from_rgb8(200, 200, 200))
            ]
            .max_width(215)
        ];
        let mut button = button(contents)
            .on_press_with(move || ModDownMsg::ModListingPressed(mods_list_index))
            .height(80)
            .width(iced::Fill)
            .padding([5, 0]);
        if let Some(a_mod) = &self.current_mod && a_mod.id == listing.project_id {
            button = button.style(|t, s| button::Style {
                background: Some(Color::from_rgb8(58, 62, 69).into()),
                text_color: Color::WHITE,
                ..button::Style::default()
            });
        }
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
        self.is_search_fetching = true;
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

        Task::perform(reqwests::get_mod_versions(id, loaders, game_versions),ModDownMsg::ModVersionsReceived).map(|m|SuperMsg(m))
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
struct ModrinthVersion {
    name: String,
    id: String,
    project_id: String,

    loaders: Vec<String>,

    version_number: String,
    changelog: String,
    dependencies: Vec<ModrinthDependency>,
    version_type: String,
}
// #[derive(Debug, serde::Deserialize)]
// struct ModrinthModFile {
//
// }

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

#[derive(Debug, PartialEq)]
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
