use std::{
    env, fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

use iced::{
    Alignment::Center, Element, Length::Fill, Task, widget::{button, column, text}
};
use zip::{ZipArchive, result::ZipError};

use crate::{Message, MinecraftVersion, ModLoader, ProgramData, REQ_CLIENT, util::circular, MC_VERSIONS};

struct PistonMetaResponse {}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct VersionsList {
    latest: _Latest,
    pub versions: Vec<MinecraftVersion>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct _Latest {
    release: String,
    snapshot: String,
}

#[derive(Clone, Debug)]
pub enum InitMessage {
    LatestVersionReceived(Result<Vec<u8>,ErrType>),
    VersionsReceived(Result<Vec<u8>,ErrType>),
    UseFileButton,
    InitConcluded,
}

#[derive(Default)]
enum InitPhase {
    #[default]
    CheckingFiles,
    CheckingLatest,
    FetchingVersions,
    Concluded,
}

#[derive(Clone,Debug)]
enum ErrType {
    ConnectionErr(String),
    StatusErr(String),
}

#[derive(Default)]
pub struct InitState {
    phase: InitPhase,
    current_path: PathBuf,
    err: Option<ErrType>,

    pub program_data: Option<ProgramData>,
    pub versions_list: Option<VersionsList>,
    pub assumed_name: String,
    pub assumed_loader: Option<ModLoader>,
    pub assumed_version: Option<MinecraftVersion>,
}
impl InitState {
    pub fn update(&mut self, _message: InitMessage) -> Task<InitMessage> {
        match _message {
            InitMessage::LatestVersionReceived(r) => {
                println!("checking latest version...");
                let b = match r {
                    Ok(b) => b,
                    Err(e) => {
                        self.err = Some(e);
                        return Task::none()
                    },
                };
                let s = String::from_utf8_lossy(&b);
                if !_is_latest_version(&s, self.versions_list.as_ref().unwrap()) {
                    self.phase = InitPhase::FetchingVersions;
                    return Task::perform(_get_minecraft_versions(), InitMessage::VersionsReceived);
                } else {
                    self._assume_program_data();
                    self.phase = InitPhase::Concluded;
                    return Task::done(InitMessage::InitConcluded);
                }
            }
            InitMessage::VersionsReceived(r) => {
                println!("fetching versions...");
                let b = match r {
                    Ok(b) => b,
                    Err(e) => {
                        self.err = Some(e);
                        return Task::none()
                    },
                };
                let des = &mut serde_json::Deserializer::from_slice(&b);
                let result: VersionsList = serde_path_to_error::deserialize(des)
                    .map_err(|e| {
                        panic!(
                            "error in versions deserialize at path {}: {e}\ndumping json: {}",
                            e.path(),
                            serde_json::to_string_pretty(
                                &serde_json::from_slice::<serde_json::Value>(&b).unwrap()
                            )
                            .unwrap()
                        )
                    })
                    .unwrap();
                let js = serde_json::to_string_pretty(&result)
                    .map_err(|e| panic!("error in json to string convert: {e}"))
                    .unwrap();
                fs::write(
                    self.current_path
                        .join(".mcservermodgui")
                        .join("minecraft_versions_list_cache.json"),
                    js,
                )
                .unwrap();
                self.versions_list = Some(result);
                
                self._assume_program_data();
                self.phase = InitPhase::Concluded;
                return Task::done(InitMessage::InitConcluded);
            },
            InitMessage::UseFileButton => {
                return Task::done(InitMessage::InitConcluded);
            }
            InitMessage::InitConcluded => unreachable!(),
        };
        Task::none()
    }

    pub fn view(&self) -> Element<InitMessage> {
        if let Some(e) = &self.err {
            let c = if self.versions_list.is_some() {
                column!["Looks like you have Minecraft versions downloaded from last time.",
                    "If you'd like, you can continue with that list.",
                    "Warning: it may be out of date and missing the very latest versions!",
                    button(text("Use that file").size(20).align_x(Center).align_y(Center)).height(50).width(150).padding(10).on_press(InitMessage::UseFileButton)
                    ].align_x(Center)
                    .padding(40)
                    .spacing(10)

            } else {column![]};
            match e {
                ErrType::ConnectionErr(s) => {
                    column![text("Error").size(18),text("Connection error! :(").size(30),text("Check your wifi. Or, Mojang's servers might be down").size(18),c]
                }
                ErrType::StatusErr(s) => {
                    column![text("Error").size(18),text("Status err! :(").size(20),text(s),c]
                }
            }
                .spacing(15)
                .padding(20)
                .align_x(Center)
                .width(Fill)
        } else {
            let label = match self.phase {
                InitPhase::CheckingFiles => "Checking cached versions",
                InitPhase::CheckingLatest => "Checking against latest versions",
                InitPhase::FetchingVersions => "Getting Minecraft versions from Mojang",
                InitPhase::Concluded => "Done!",
            };
            column![text("Starting app").size(18),text(label).size(30), circular::Circular::new()]
                .spacing(15)
                .padding(20)
                .align_x(Center)
                .width(Fill)
        }.into()
    }

    pub fn init(&mut self) -> Task<InitMessage> {
        println!("** INIT:");
        self.current_path = env::current_dir().unwrap();

        let m = self.current_path.join(".mcservermodgui");
        if Path::is_dir(&m) {
            let d = m.join("mcservermodgui.toml");
            if Path::is_file(&d) {
                let b = fs::read(d).unwrap();
                let result: Result<ProgramData, _> = toml::from_slice(&b);
                self.program_data = Some(result.unwrap());
            }
            let v = m.join("minecraft_versions_list_cache.json");
            if Path::is_file(&v) {
                let b = fs::read(v).unwrap();
                let des = &mut serde_json::Deserializer::from_slice(&b);
                let result: VersionsList = serde_path_to_error::deserialize(des)
                    .unwrap_or_else(|e| panic!("error in list cache at path {}: {e}", e.path()));

                self.versions_list = Some(result);
                self.phase = InitPhase::CheckingLatest;
                return Task::perform(
                    _get_latest_minecraft_version(),
                    InitMessage::LatestVersionReceived,
                );
            } else {
                self.phase = InitPhase::CheckingFiles;
                return Task::perform(_get_minecraft_versions(), InitMessage::VersionsReceived);
            }
        } else {
            self.phase = InitPhase::CheckingFiles;
            fs::create_dir(m)
                .map_err(|e| panic!("error in directory creation: {e}"))
                .unwrap();
            return Task::perform(_get_minecraft_versions(), InitMessage::VersionsReceived);
        }
    }

    fn _assume_program_data(&mut self) {
        self.assumed_name = self.current_path.file_name().expect("CRITICAL: COULDN'T ACCESS PARENT FOLDER").to_string_lossy().to_owned().to_string();
        let loader:ModLoader =
        if Path::is_dir(&self.current_path.join("plugins")) {
            if Path::is_dir(&self.current_path.join("libraries").join("dev").join("folia")) {
                ModLoader::Folia
            } else {
                if Path::is_file(&self.current_path.join("purpur.yml")) {
                    ModLoader::Purpur
                } else if Path::is_file(&self.current_path.join("velocity.toml")) {
                    self.assumed_loader = Some(ModLoader::Velocity);
                    return
                } else {
                    ModLoader::Paper
                }
            }
        } else {
            if Path::is_dir(&self.current_path.join(".fabric")) {
                ModLoader::Fabric
            }
            else if Path::is_file(&self.current_path.join("config").join("neoforge-common.toml")) {
                ModLoader::NeoForge
            }
            else if Path::is_file(&self.current_path.join("config").join("forge-common.toml")) {
                ModLoader::Forge
            } else {
                return
            }
        };
        self.assumed_loader = Some(loader);
        let path = match loader {
            ModLoader::Fabric | ModLoader::Paper | ModLoader::Purpur | ModLoader::Folia => self.current_path.join("versions"),
            ModLoader::NeoForge | ModLoader::Forge => self.current_path.join("libraries").join("net").join("minecraft").join("server"),
            ModLoader::Velocity => unreachable!()
        };
        let mut r = match fs::read_dir(path) {
            Ok(o) => o,
            Err(e) => return
        };
        let Some(folder) = r.next() else {return};
        let ver = folder.unwrap().file_name().to_string_lossy().into_owned();
        let found = self.versions_list.as_ref().unwrap().versions.iter().find(|v| *v.id == ver).cloned();
        self.assumed_version = found;
    }
}

async fn _get_latest_minecraft_version() -> Result<Vec<u8>,ErrType> {
    let b_client = &REQ_CLIENT;
    let get = b_client
        .get("https://piston-meta.mojang.com/mc/game/version_manifest.json")
        .header(reqwest::header::RANGE, "bytes=12-128");
    let response = match get.send().await {
        Ok(o) => o,
        Err(e) => {
            if e.is_connect() { return Err(ErrType::ConnectionErr(e.to_string())) }
            if e.is_status() { return Err(ErrType::StatusErr(e.status().map(|v|v.to_string()).unwrap_or_else(|| "Could not get status code :(".to_string())))}
            panic!("Unhandled error in request: {}",e.to_string())
        }
    };
    
    if !response.status().is_success() {
        Err(ErrType::StatusErr(response.error_for_status().unwrap_err().to_string()))
    } else {
        let bytes = response
            .bytes()
            .await
            .expect("error turning the recieved data to bytes").to_vec();
        Ok(bytes)
    }
}

async fn _get_minecraft_versions() -> Result<Vec<u8>,ErrType> {
    let b_client = &REQ_CLIENT;
    let get = b_client.get("https://piston-meta.mojang.com/mc/game/version_manifest.json");
    let response = match get.send().await {
        Ok(o) => o,
        Err(e) => {
            if e.is_connect() { return Err(ErrType::ConnectionErr(e.to_string())) }
            if e.is_status() { return Err(ErrType::StatusErr(e.status().map(|v|v.to_string()).unwrap_or_else(|| "Could not get status code :(".to_string())))}
            panic!("Unhandled error in request: {}",e.to_string())
        }
    };
    
    if !response.status().is_success() {
        Err(ErrType::StatusErr(response.error_for_status().unwrap_err().to_string()))
    } else {
        let bytes = response
            .bytes()
            .await
            .expect("error turning the recieved data to bytes").to_vec();
        Ok(bytes)
    }
}

// async fn init() -> Result<(Vec<MinecraftVersion>,Option<ProgramData>), Box<dyn std::error::Error>> {

//     let current_path = env::current_dir()?;
//     println!("current directory: {}",current_path.display());

//     let mut program_data: Option<ProgramData> = None;
//     let mut versions: Option<Vec<MinecraftVersion>> = None;
//     let mut has_folder = false;

//     for entry in fs::read_dir(&current_path)? {
//         let entry = entry?;
//         println!("*) {}", entry.file_name().to_string_lossy());
//         if entry.file_type()?.is_dir() && entry.file_name() == ".mcservermodgui" {
//             has_folder = true;
//             for e in fs::read_dir(entry.path())? {
//                 let e = e?;
//                 if e.file_name() == "mcservermodgui.toml" {
//                     let b = fs::read(e.path())?;
//                     let result: Result<ProgramData,_>  = toml::from_slice(&b);
//                     program_data = Some(result?);
//                 }
//                 if e.file_name() == "minecraft_versions_list_cache.json" {
//                     let b = fs::read(e.path())?;
//                     let des = &mut serde_json::Deserializer::from_slice(&b);
//                     let result: VersionsList = serde_path_to_error::deserialize( des)
//                         .map_err(|e| panic!("error in list cache at path {}: {e}",e.path())).unwrap();

//                     if _is_latest_version(&result) {
//                         versions = Some(result.versions);
//                     } // if isn't latest version, 'versions' will stay None and get re-scanned anyway
//                 }
//             }
//         }
//         if let Some(filetype) = entry.path().extension() && filetype == "jar" {
//             println!("jar");
//             _unzip_in_memory(fs::read(entry.path())?);
//         }
//     }

//     if !has_folder {
//         fs::create_dir(current_path.join(".mcservermodgui")).map_err(|e| panic!("error in directory creation: {e}")).unwrap();
//     }
//     if versions.is_none() {
//         println!("is none");
//         let b = _get_minecraft_versions().map_err(|e|panic!("mojang responded with {e}")).unwrap();
//         let des = &mut serde_json::Deserializer::from_slice(&b);
//         let result: VersionsList = serde_path_to_error::deserialize( des)
//             .map_err(|e| panic!("error in versions deserialize at path {}: {e}\ndumping json: {}",
//             e.path(),serde_json::to_string_pretty(&serde_json::from_slice::<serde_json::Value>(&b).unwrap()).unwrap())).unwrap();
//         let js = serde_json::to_string_pretty(&result).map_err(|e| panic!("error in json to string convert: {e}")).unwrap();
//         fs::write(current_path.join(".mcservermodgui").join("minecraft_versions_list_cache.json"), js)?;
//         versions = Some(result.versions);
//     }

//     Ok((versions.unwrap(), program_data))

//     // match result {
//     //     Ok(response) => {
//     //         // println!("{:#?}",response.versions);
//     //         return Ok((response.versions, program_data));
//     //     }
//     //     Err(e) => {
//     //         let path = e.path().to_string();
//     //         panic!("error parsing JSON from search: {path}\ndumping json: {}",String::from_utf8(b).unwrap_or_else(|e| e.to_string()))
//     //     }
//     // };
// }

// fn _unzip_in_memory(file_contents: Vec<u8>) -> Result<(),ZipError> {
//     let mut buffer = Vec::new();

//     let cursor = Cursor::new(file_contents);

//     let mut archive = ZipArchive::new(cursor)?;

//     for i in 0..archive.len() {
//         let mut f = archive.by_index(i)?;
//         if f.name() == "install.properties" {
//             println!("install.properties");
//             f.read_to_end(&mut buffer)?;
//         }
//     }

//     let s = String::from_utf8_lossy(&buffer);
//     println!("{s}");
//     Ok(())
// }

fn _is_latest_version(response: &str, list: &VersionsList) -> bool {
    // let b = _get_latest_minecraft_version().await.map_err(|e|panic!("error getting latest mc version: {e}")).unwrap();
    // let s = String::from_utf8_lossy(&b);

    let s1 = response.replace(r#""release": ""#, "");

    let i1 = s1.find('"').unwrap();
    let latest_release = &s1[..i1];
    println!("latest release: {latest_release}");

    let s2 = s1.replace(
        format!("{}\", \"snapshot\": \"", latest_release).as_str(),
        "",
    );

    let i2 = s2.find('"').unwrap();
    let latest_snapshot = &s2[..i2];
    println!("latest snapshot: {latest_snapshot}");

    return latest_release == &list.latest.release && latest_snapshot == &list.latest.snapshot;
}
