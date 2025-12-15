use std::{collections::{HashMap, HashSet}, f32::consts::PI, sync::{Arc, LazyLock}};

use iced::{Alignment, Color, Degrees, Element, Gradient, Radians, Task, advanced::{image::Handle as RasterHandle, svg::Handle as SvgHandle, widget::operation}, color, gradient::Linear, widget::{self, Button, button, column, container, row, scrollable::{self, Viewport}, text, text_input}, window};
use frostmark::{MarkState, MarkWidget};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use smart_default::{self, SmartDefault};

use crate::{ImageType, MISSING_IMAGE, Message::{self, ModDLMessage as SuperMsg}, reqwests};


#[derive(Debug,Clone)]
pub enum ModDownMsg {
    OpenLink(String),

    ImageDownloaded(Result<reqwests::ImageData, String>),
    ModsSearchReceived(Result<Vec<u8>, String>),
    ModRecieved(Result<Vec<u8>, String>),
    ModListingPressed(usize),
    ModsListScrolled(Viewport),
    SearchTyped(String),
    SearchSubmitted,
}

#[derive(SmartDefault)]
pub struct ModDownloader {
    #[default(MarkState::with_html(" "))]
    markup_state: MarkState,

    cached_images: HashMap<String, ImageType>,
    images_queued: HashSet<String>,

    mods_cached: HashMap<String,ModrinthMod>,
    selected_mod: String,
    current_searchbar_text: String,
    current_query: String,
    mods_search_results: Vec<ModrinthSearchResult>,
    mods_search_offset: u64,
    
    scroll_load_debounce: bool,

    is_search_fetching: bool,
}
impl ModDownloader {
    pub fn update(&mut self, _message:ModDownMsg) -> Task<Message> {
        match _message {
            ModDownMsg::OpenLink(url) => return Task::done(Message::OpenLink(url)),

            ModDownMsg::ImageDownloaded(res) => match res{
                Ok(img) => {
                    if img.is_svg {
                        self.cached_images.insert(img.url, ImageType::Svg(SvgHandle::from_memory(img.bytes)));
                    } else {
                        self.cached_images.insert(img.url, ImageType::Raster(RasterHandle::from_bytes(img.bytes)));
                    }
                }
                Err(err) => {eprintln!("Couldn't download image: {err}");}
            },
            ModDownMsg::ModsSearchReceived(res) => match res{
                Ok(val) => {
                    self.is_search_fetching = false;
                    let des = &mut serde_json::Deserializer::from_slice(&val);
                    let result: Result<ModrinthSearch, _> = serde_path_to_error::deserialize( des);
                    match result {
                        Ok(search) => {
                            let mut img_tasks: Vec<Task<Message>> = vec![];
                            // println!("{:?}",search);
                            for r in search.hits {
                                if let Some(url) = &r.icon_url {
                                    img_tasks.push(Task::perform(reqwests::download_image(url.clone()),|r| SuperMsg(ModDownMsg::ImageDownloaded(r))));
                                }
                                self.mods_search_results.push(r);
                            }
                            self.scroll_load_debounce = false;
                            return Task::batch(img_tasks);
                        }
                        Err(e) => {
                            let path = e.path().to_string();
                            eprintln!("error parsing JSON from search: {path}\ndumping json: {}",String::from_utf8(val).unwrap_or_else(|e| e.to_string()))
                        }
                    };
                }
                Err(err) => {eprintln!("Couldn't get json: {err}");}
            },
            ModDownMsg::ModListingPressed(index) => {
                let task = scrollable::scroll_to(scrollable::Id::new("markup"), scrollable::AbsoluteOffset::default());
                let slug = &self.mods_search_results[index].slug;
                self.selected_mod = slug.clone();
                if let Some(modrinth_mod) = self.mods_cached.get(slug) {
                    self.markup_state = MarkState::with_html_and_markdown(&modrinth_mod.body);
                    return Task::batch([self._download_markup_images(),task]).map(|m|SuperMsg(m));
                } else {
                    self.markup_state = MarkState::with_html("loading...");
                    return Task::batch([Task::perform(reqwests::fetch_mod(slug.clone()), ModDownMsg::ModRecieved),task]).map(|m|SuperMsg(m));
                }
            },
            ModDownMsg::ModRecieved(res) => match res {
                Ok(val) => {
                    let des = &mut serde_json::Deserializer::from_slice(&val);
                    let result: Result<ModrinthMod, _> = serde_path_to_error::deserialize( des);
                    match result {
                        Ok(m) => {
                            let slug = m.slug.clone();
                            if self.selected_mod == slug {
                                self.markup_state = MarkState::with_html_and_markdown(&m.body);
                                self.mods_cached.insert(slug,m);
                                return self._download_markup_images().map(|m|SuperMsg(m))
                            }
                            self.mods_cached.insert(slug,m);
                        }
                        Err(e) => {
                            let path = e.path().to_string();
                            eprintln!("error parsing JSON from mod fetch: {} path: {path}\ndumping json: {}",e,String::from_utf8(val).unwrap_or_else(|e| e.to_string()))
                        }
                    };
                },
                Err(err) => {eprintln!("Couldn't get json: {err}");}
            }
            ModDownMsg::ModsListScrolled(viewport) => {
                if viewport.absolute_offset_reversed().y <= 300.0 {
                    if !self.scroll_load_debounce {
                        self.mods_search_offset += 20;
                        self.scroll_load_debounce = true;
                        return Task::perform(reqwests::search_mods(self.mods_search_offset, None, Some(vec!["server_side!=unsupported".to_string()])),ModDownMsg::ModsSearchReceived).map(|m|SuperMsg(m));
                    }
                }// else {println!("debounce failed {}",test);}
                // } else {
                //     println!("turning off debounce");
                //     self.scroll_load_debounce = false
                // }
                // println!("scroll: {:?} offset_reversed: {:?}",viewport, viewport.absolute_offset_reversed());
            },
            ModDownMsg::SearchTyped(s) => {
                self.current_searchbar_text = s;
            }
            ModDownMsg::SearchSubmitted => {
                if self.current_searchbar_text == self.current_query {return Task::none()}
                self.current_query = self.current_searchbar_text.clone();
                return self._new_mod_search(Some(vec!["server_side!=unsupported".to_string()])).map(|m|SuperMsg(m));
            }
        };
        Task::none()
    }
    
    pub fn view(&self) -> Element<ModDownMsg> {
        column![
            text_input("Search...", &self.current_searchbar_text).on_input(|s| ModDownMsg::SearchTyped(s)).on_submit(ModDownMsg::SearchSubmitted).icon(text_input::Icon { font: iced::Font::DEFAULT, code_point: '⌕', size: None, spacing: 4.0, side: text_input::Side::Left }),
            row![
                widget::scrollable(// mod list
                    if self.mods_search_results.is_empty() {
                        if self.is_search_fetching {
                            iced::Element::from(container("loading...").center(100))
                        } else {
                            container("no mods found :(").center(100).into()
                        }
                    } else {
                        column((0..self.mods_search_results.len()).into_iter().map(|i| self._create_mod_listing(i))).spacing(5).into()
                    }
                ).width(320).spacing(5).on_scroll(ModDownMsg::ModsListScrolled).id(scrollable::Id::new("search")),
                widget::scrollable( // markdown section
                    if !self.selected_mod.is_empty() && let Some(listing) = self.mods_cached.get(&self.selected_mod) {
                        const IMG_SIZE:u16 = 100;
                        let thumbnail: Element<ModDownMsg> = if let Some(url) = &listing.icon_url {
                            if let Some(img) = self.cached_images.get(url).cloned() {
                                match img {
                                    ImageType::Svg(handle) => widget::svg(handle).width(IMG_SIZE).height(IMG_SIZE).into(),
                                    ImageType::Raster(handle) => widget::image(handle).width(IMG_SIZE).height(IMG_SIZE).into(),
                                }
                            } else {
                                eprintln!("Image not found in cache for search builder! Creating default image");
                                widget::image(&*MISSING_IMAGE).width(IMG_SIZE).height(IMG_SIZE).into()
                            }
                        } else {
                            eprintln!("nonexistent thumbnail in search builder");
                            widget::Image::new(&*MISSING_IMAGE).width(IMG_SIZE).height(IMG_SIZE).into()
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
                            MarkWidget::new(&self.markup_state).on_clicking_link(|url| ModDownMsg::OpenLink(url))
                                .on_drawing_image(|info| {
                                    if let Some(image) = self.cached_images.get(info.url).cloned() {
                                        match image {
                                            ImageType::Svg(handle) => {
                                                let mut img = widget::svg(handle);
                                                if let Some(w) = info.width {
                                                    img = img.width(w)
                                                }
                                                if let Some(h) = info.height {
                                                    img = img.height(h);
                                                }
                                                img.into()
                                            }
                                            ImageType::Raster(handle) => {
                                                let mut img = widget::image(handle);
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
                                        widget::image(&*MISSING_IMAGE).width(128).height(128).into()
                                    }
                                }
                            )
                        ].spacing(10).into()
                    } else {Element::from("")}
                ).width(iced::Fill).spacing(5).id(scrollable::Id::new("markup"))
            ].spacing(10)
        ].spacing(10).padding(15).into()
    }

    pub fn init(&mut self) -> Task<Message> {
        self._new_mod_search(Some(vec!["server_side!=unsupported".to_string()])).map(|m|SuperMsg(m))
    }

    fn _create_mod_listing(&'_ self, mods_list_index:usize) -> iced::Element<'_, ModDownMsg> {
        const IMG_SIZE:u16 = 75;
        let listing = &self.mods_search_results[mods_list_index];
        let thumbnail: Element<ModDownMsg> = if let Some(url) = &listing.icon_url {
            if let Some(img) = self.cached_images.get(url).cloned() {
                match img {
                    ImageType::Svg(handle) => widget::svg(handle).width(IMG_SIZE).height(IMG_SIZE).into(),
                    ImageType::Raster(handle) => widget::image(handle).width(IMG_SIZE).height(IMG_SIZE).into(),
                }
            } else {
                eprintln!("Image not found in cache for search builder! Creating default image");
                widget::image(&*MISSING_IMAGE).width(IMG_SIZE).height(IMG_SIZE).into()
            }
        } else {
            eprintln!("nonexistent thumbnail in search builder");
            widget::Image::new(&*MISSING_IMAGE).width(IMG_SIZE).height(IMG_SIZE).into()
        };
        let contents = row![container(thumbnail).padding([0,5]), column![listing.title.as_str(), text(&listing.description).size(10).color(Color::from_rgb8(200, 200, 200))].max_width(215)];
        let mut button = button(contents).on_press_with(move || ModDownMsg::ModListingPressed(mods_list_index)).height(80).width(iced::Fill).padding([5, 0]);
        if self.selected_mod == listing.slug {
            button = button.style(|t,s| button::Style {
                background: Some(Color::from_rgb8(58, 62, 69).into()),
                text_color: Color::WHITE,
                ..button::Style::default()
            });
        }
        button.into()
    }

    fn _download_markup_images(&mut self) -> Task<ModDownMsg> {
        Task::batch(self.markup_state.find_image_links().into_iter().map(|url| {
            if self.images_queued.insert(url.clone()) {
                Task::perform(reqwests::download_image(url), ModDownMsg::ImageDownloaded)
            } else {
                Task::none()
            }
        }))
    }

    fn _new_mod_search(&mut self, args: Option<Vec<String>>) -> Task<ModDownMsg> {
        self.is_search_fetching = true;
        self.mods_search_results.clear();
        self.mods_search_offset = 0;
        println!("{:?}",self.mods_search_results);
        Task::batch([
            Task::perform(reqwests::search_mods(self.mods_search_offset, Some(self.current_query.clone()), args),ModDownMsg::ModsSearchReceived),
            scrollable::scroll_to(scrollable::Id::new("search"), scrollable::AbsoluteOffset::default())])
    }

}

#[derive(Debug,serde::Deserialize)]
struct ModrinthMod {
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

#[derive(Debug,serde::Deserialize)]
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

#[derive(Debug,serde::Deserialize)]
struct ModrinthSearch {
    hits: Vec<ModrinthSearchResult>,
    offset: i64,
    limit: i64,
    total_hits: i64,
}

fn mmod_color_handler<'de, D>(
    deserializer: D,
) -> Result<Option<Color>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let Some(v):Option<i64> = Deserialize::deserialize(deserializer)? else {return Ok(None)};

    let b = (v % 256).try_into().map_err(serde::de::Error::custom)?;
    let g = ((v >> 8) % 256).try_into().map_err(serde::de::Error::custom)?;
    let r = ((v >> 16) % 256).try_into().map_err(serde::de::Error::custom)?;

    Ok(Some(Color::from_rgb8(r, g, b)))
}
