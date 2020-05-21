use std::{
    collections::HashSet,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    process::Command,
};

use indexmap::IndexSet;
use pathdiff::diff_paths;
use serde_derive::{Deserialize, Serialize};
use walkdir::{DirEntry, WalkDir};

use crate::{
    fomod,
    library::{self, AndCreateDirs},
    utils,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_game: Option<String>,
    #[serde(default)]
    pub games: HashSet<String>,
}

impl GlobalConfig {
    pub fn open() -> crate::Result<Self> {
        match fs::read(library::global_config()?) {
            Ok(bytes) => toml::from_slice(&bytes).map_err(Into::into),
            Err(_) => Ok(Self::default()),
        }
    }
    pub fn save(&self) -> crate::Result<()> {
        let string = toml::to_string_pretty(self)?;
        fs::write(library::global_config()?, &string).map_err(Into::into)
    }
    pub fn init_game(
        &mut self,
        name: String,
        folder: PathBuf,
        data: Option<PathBuf>,
    ) -> crate::Result<()> {
        if self.games.contains(&name) {
            return Err(crate::Error::AlreadyManaged(name));
        }
        self.active_game.get_or_insert_with(|| name.clone());
        self.games.insert(name.clone());
        Game {
            name: name.clone(),
            config: Config {
                data_folder: data,
                game_folder: folder,
                enabled: IndexSet::new(),
                disabled: HashSet::new(),
            },
        }
        .save()?;
        library::downloads_dir(&name)?;
        println!("Climm initialized {}", name);
        Ok(())
    }
    pub fn game(&self, name: &str) -> crate::Result<Game> {
        if !self.games.contains(name) {
            return Err(crate::Error::UnknownGame(name.into()));
        }
        Game::open(name)
    }
    pub fn active_game(&self) -> crate::Result<Game> {
        self.game(
            self.active_game
                .as_deref()
                .ok_or(crate::Error::NoActiveGame)?,
        )
    }
}

impl Drop for GlobalConfig {
    fn drop(&mut self) {
        if let Err(e) = self.save() {
            println!("Error saving global config: {}", e);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub game_folder: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_folder: Option<PathBuf>,
    #[serde(default)]
    pub enabled: IndexSet<String>,
    #[serde(default)]
    pub disabled: HashSet<String>,
}

pub struct Game {
    name: String,
    config: Config,
}

const GAME_CONFIG_FILE: &str = "climm.toml";

fn game_config_file(name: &str) -> crate::Result<PathBuf> {
    library::game_dir(name).map(|game_dir| game_dir.join(GAME_CONFIG_FILE))
}

impl Game {
    pub fn config_file(&self) -> crate::Result<PathBuf> {
        game_config_file(&self.name)
    }
    pub fn install_dir(&self) -> PathBuf {
        if let Some(data) = &self.config.data_folder {
            self.config.game_folder.join(data)
        } else {
            self.config.game_folder.clone()
        }
    }
    pub fn open(name: &str) -> crate::Result<Self> {
        let bytes = fs::read(game_config_file(name)?)?;
        let config: Config = toml::from_slice(&bytes)?;
        Ok(Game {
            name: name.into(),
            config,
        })
    }
    pub fn save(&self) -> crate::Result<()> {
        let string = toml::to_string_pretty(&self.config)?;
        fs::write(self.config_file()?, &string).map_err(Into::into)
    }
    fn extract(&mut self) -> crate::Result<()> {
        for entry in fs::read_dir(library::downloads_dir(&self.name)?)? {
            let entry = entry?;
            // If the entry is a file
            if entry.file_type()?.is_file() {
                // Get the mod name
                let mod_name = mod_name(entry.path()).unwrap();
                // Get the extracted dir
                let extracted_dir = library::extracted_dir(&self.name, &mod_name)?;
                // Check if the mod should be installed
                let should_be_installed = !self.config.disabled.contains(&mod_name);
                // Check if any files from the mod are installed
                let is_extracted = extracted_dir.exists();
                let extracted_dir = extracted_dir.and_create_dirs::<crate::Error>()?;
                // Extract if necessary
                if should_be_installed && !is_extracted {
                    utils::print_erasable(&format!("Extracting {:?}", mod_name));
                    if Command::new("7z")
                        .arg("x")
                        .arg(entry.path())
                        .arg(format!("-o{}", extracted_dir.to_string_lossy()))
                        .output()?
                        .status
                        .success()
                    {
                        // Mark mod as enabled
                        self.config.enabled.insert(mod_name.clone());
                    } else {
                        utils::remove_path(&extracted_dir, "")?;
                    }
                }
            }
        }
        Ok(())
    }
    fn install(&mut self) -> crate::Result<()> {
        let install_dir = self.install_dir();
        for enabled in &self.config.enabled {
            let enabled_lower = enabled.to_lowercase();
            let mod_path = fs::read_dir(library::extracted_dir(&self.name, "")?)?
                .filter_map(Result::ok)
                .find(|entry| {
                    entry.path().is_dir()
                        && entry
                            .path()
                            .to_string_lossy()
                            .to_lowercase()
                            .contains(&enabled_lower)
                })
                .map(|entry| entry.path())
                .ok_or_else(|| crate::Error::UnknownArchive(enabled.clone()))?;
            // Get the mod name
            let mod_name = mod_path
                .iter()
                .last()
                .expect("dir entry has empty path")
                .to_string_lossy()
                .into_owned();
            // Check if the mod should be installed
            let should_be_installed = !self.config.disabled.contains(&mod_name);

            // Install if necessary
            if should_be_installed {
                // Check for fomod
                let config = WalkDir::new(&mod_path)
                    .into_iter()
                    .filter_map(Result::ok)
                    .find(|entry| {
                        entry
                            .path()
                            .file_name()
                            .map_or(false, |name| name == "ModuleConfig.xml")
                    })
                    .map(DirEntry::into_path);
                let install_paths = if config.is_some() {
                    fomod::pseudo_fomod(&mod_path)?
                } else {
                    vec![mod_path]
                };
                // For each folder
                for path in install_paths {
                    let mod_diff = differ(&path);
                    // For each file
                    for entry in WalkDir::new(&path) {
                        let file_entry = entry?;
                        if file_entry.file_type().is_file() {
                            let extracted_path = path.join(mod_diff(&file_entry).unwrap());
                            let install_path = install_dir.join(mod_diff(&file_entry).unwrap());
                            utils::create_dirs(&install_path)?;
                            let mut extracted_file = File::open(extracted_path)?;
                            let mut install_file = File::create(install_path)?;
                            io::copy(&mut extracted_file, &mut install_file)?;
                        }
                    }
                }
                println!("Installed {:?} ", mod_name);
            }
            // Uninstall if necessary
            else {
                let mod_diff = differ(&mod_path);
                for entry in WalkDir::new(&mod_path) {
                    let file_entry = entry?;
                    utils::remove_path(&install_dir, mod_diff(&file_entry).unwrap())?;
                }
                println!("Uninstalled {:?}", mod_name);
            }
        }
        Ok(())
    }
    pub fn update(&mut self) -> crate::Result<()> {
        self.extract()?;
        self.install()?;
        Ok(())
    }
    pub fn clean(&mut self) -> crate::Result<()> {
        let install_dir = self.install_dir();
        // Extract downloads
        for entry in fs::read_dir(library::downloads_dir(&self.name)?)? {
            let entry = entry?;
            // If the entry is a file
            if entry.file_type()?.is_file() {
                // Get the mod name
                let mod_name = mod_name(entry.path()).unwrap();
                // Get the extracted dir
                let extracted_dir = library::extracted_dir(&self.name, &mod_name)?;
                let mod_diff = differ(&extracted_dir);
                // Delete if necessary
                if !(self.config.enabled.contains(&mod_name)
                    || self.config.disabled.contains(&mod_name))
                {
                    for entry in WalkDir::new(&extracted_dir) {
                        let file_entry = entry?;
                        utils::remove_path(&install_dir, mod_diff(&file_entry).unwrap())?;
                    }
                    utils::remove_path(&extracted_dir, "")?;
                    fs::remove_file(entry.path())?;
                    println!("Deleted {:?}", mod_name);
                }
            }
        }
        Ok(())
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        if let Err(e) = self.save() {
            println!("Error saving config: {}", e);
        }
    }
}

fn mod_name<P>(file: P) -> Option<String>
where
    P: AsRef<Path>,
{
    file.as_ref()
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
}

fn differ<P>(top: &P) -> impl Fn(&'_ DirEntry) -> Option<PathBuf> + '_
where
    P: AsRef<Path>,
{
    move |entry| diff_paths(entry.path(), top)
}
