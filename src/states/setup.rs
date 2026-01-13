use std::{sync::OnceLock, time::Duration};
use std::iter::Iterator;
use std::sync::LazyLock;
use iced::{
    Alignment::Center, Border, Color, Element, Font, Length::Fill, Task, Theme, alignment::Horizontal::Right, widget::{Space, button, checkbox, column, container, pick_list, right, row, text, text_input}
};
use iced::advanced::svg::Svg;
use strum::{IntoEnumIterator, VariantArray};

use crate::{Message, MinecraftVersion, ModLoader, ProgramData, VersionKind, circular::Circular, MC_VERSIONS, STATIC_IMAGES, SVG_MOD_LOADERS};
use crate::util::icon_pick_list::icon_pick_list;

#[derive(Clone, Debug)]
pub enum SetupMessage {
    NameTyped(String),
    McVersionSelected(MinecraftVersion),
    ShowSnapshotsToggled(bool),
    LoaderSelected(ModLoader),
    DoneButtonPressed,
    DoNothing,
    SetupConcluded,
}

#[derive(Default)]
pub struct SetupState {
    pub name: String,
    pub selected_loader: Option<ModLoader>,
    pub selected_version: Option<MinecraftVersion>,
    show_snapshots: bool,
    version: String,
    error: String,

    pub program_data: OnceLock<ProgramData>,
}
impl SetupState {
    pub fn update(&mut self, _message: SetupMessage) -> Task<Message> {
        match _message {
            SetupMessage::NameTyped(s) => self.name = s,
            SetupMessage::McVersionSelected(s) => self.selected_version = Some(s),
            SetupMessage::ShowSnapshotsToggled(b) => self.show_snapshots = b,
            SetupMessage::LoaderSelected(s) => self.selected_loader = Some(s),
            SetupMessage::DoneButtonPressed => {
                if self.name.trim().is_empty() {
                    self.error = "Enter a name".to_string();
                    return Task::none();
                }
                let Some(loader) = self.selected_loader else {
                    self.error = "Select a mod loader".to_string();
                    return Task::none();
                };
                let Some(version) = self.selected_version.clone() else {
                    self.error = "Select a Minecraft version".to_string();
                    return Task::none();
                };
                // let version = if let ModLoader::Velocity = loader { None } else {
                //     if let Some(v) = &self.selected_version { Some(v.clone()) } else {
                //         self.error = "Select a Minecraft version".to_string();
                //         return  Task::none()
                //     }
                // };
                self.program_data.set(ProgramData {
                    name: self.name.clone(),
                    loader,
                    version
                }).expect("Attempted to write to program_data twice");
                return Task::done(Message::SetupMessage(SetupMessage::SetupConcluded))
            },
            SetupMessage::SetupConcluded | SetupMessage::DoNothing => ()
        }
        Task::none()
    }
    pub fn view(&self) -> Element<'_, SetupMessage> {
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

        const SEPARATION_SPACING: u32 = 150;
        container(column![
            text("Configure Server").size(30).align_x(Center).width(Fill),
            text(&self.error).color(Color::from_rgb8(200, 0, 0)),//.size(15),
            row![
                text("Name: ").width(SEPARATION_SPACING),
                text_input("Minecraft server", &self.name).on_input(SetupMessage::NameTyped)
            ].align_y(Center),
            row![text("Loader: ").width(SEPARATION_SPACING),
                icon_pick_list(ModLoader::VARIANTS, self.selected_loader, SetupMessage::LoaderSelected,
                    |l| vec![match l {
                        ModLoader::Fabric => SVG_MOD_LOADERS[1].clone(),
                        ModLoader::NeoForge => SVG_MOD_LOADERS[2].clone(),
                        ModLoader::Forge => SVG_MOD_LOADERS[3].clone(),
                        ModLoader::Paper => SVG_MOD_LOADERS[4].clone(),
                        ModLoader::Purpur => SVG_MOD_LOADERS[5].clone(),
                        ModLoader::Folia => SVG_MOD_LOADERS[6].clone(),
                        ModLoader::Velocity => SVG_MOD_LOADERS[7].clone(),
                        _ => SVG_MOD_LOADERS[0].clone(),
                    }],
                ).width(Fill)
            ].align_y(Center),
            column![
                row![text("Minecraft Version: ").width(SEPARATION_SPACING),
                    pick_list(MC_VERSIONS.get().unwrap().iter().filter(filter).cloned().collect::<Vec<_>>(), self.selected_version.clone(), SetupMessage::McVersionSelected).width(Fill)
                ].align_y(Center),
                row![Space::new().width(SEPARATION_SPACING), checkbox(self.show_snapshots).label("Show snapshots").on_toggle(SetupMessage::ShowSnapshotsToggled)
                    .style(|_, _| checkbox::Style {
                        icon_color: Color::WHITE,
                        text_color: Some(Color::from_rgb8(180, 180, 180)),
                        
                        background: iced::Background::Color(Color::TRANSPARENT),
                        border: Border::default().color(Color::WHITE).rounded(2).width(1)
                    }
                ).size(13).text_size(15).spacing(7)]
            ].spacing(6),
            right(button(text("Ok").align_x(Center).align_y(Center)).width(100).height(35).on_press(SetupMessage::DoneButtonPressed))
        ].max_width(500).spacing(10)).padding(40).align_x(Center)
        .into()
    }
}
