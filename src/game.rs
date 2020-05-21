use std::{
    collections::HashSet,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
};

use indexmap::IndexSet;
use serde_derive::{Deserialize, Serialize};
use walkdir::WalkDir;
use zip::ZipArchive;

use crate::{fomod, library, utils};

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
        let install_dir = self.install_dir();
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
                // Load the archive
                let mut archive = ZipArchive::new(File::open(entry.path())?)?;
                // Check if any files from the mod are installed
                let any_installed = archive
                    .file_names()
                    .any(|name| install_dir.join(name).exists());
                // Install if necessary
                if should_be_installed && !any_installed {
                    utils::print_erasable(&format!("Extracting {:?}", mod_name));
                    // For each file in the archive
                    for i in 0..archive.len() {
                        let mut zipped_file = archive.by_index(i)?;
                        let extracted_path = extracted_dir.join(zipped_file.name());
                        let install_path = install_dir.join(zipped_file.name());
                        // Extract the file if it does not exist
                        if !install_path.exists() {
                            utils::create_dirs(&install_path)?;
                            // Extract from the archive if necessary
                            if !extracted_path.exists() {
                                let mut extracted_file = File::create(extracted_path)?;
                                io::copy(&mut zipped_file, &mut extracted_file)?;
                            }
                        }
                    }
                    self.config.enabled.insert(mod_name.clone());
                }
                // Uninstall if necessary
                if !should_be_installed && any_installed {
                    for name in archive.file_names() {
                        utils::remove_path(&install_dir, name)?;
                    }
                    println!("Uninstalled {:?}", mod_name);
                }
            }
        }
        Ok(())
    }
    fn install(&mut self) -> crate::Result<()> {
        let install_dir = self.install_dir();
        for entry in fs::read_dir(library::extracted_dir(&self.name, "")?)? {
            let mod_entry = entry?;
            if !mod_entry.file_type()?.is_dir() {
                continue;
            }
            // Get the mod name
            let mod_name = mod_entry
                .path()
                .iter()
                .last()
                .expect("dir entry has empty path")
                .to_string_lossy()
                .into_owned();
            // Check for fomod
            if let Some(fomod_path) = ["fomod", "Fomod"]
                .iter()
                .map(|name| mod_entry.path().join(name))
                .find(|path| path.is_dir())
            {
                if let (Ok(info_file), Ok(config_file)) = (
                    File::open(fomod_path.join("info.xml")),
                    File::open(fomod_path.join("ModuleConfig.xml")),
                ) {
                    fomod::Fomod::parse(info_file, config_file)?;
                }
                return Ok(());
            }
            // Check if the mod should be installed
            let should_be_installed = !self.config.disabled.contains(&mod_name);
            // Check if any files from the mod are installed
            let any_installed = walkdir::WalkDir::new(mod_entry.path())
                .into_iter()
                .filter_map(Result::ok)
                .any(|entry| {
                    entry.path();
                    let is_file = entry.file_type().is_file();
                    let exists = install_dir.join(entry.file_name()).exists();
                    is_file && exists
                });
            if should_be_installed && !any_installed {
                utils::print_erasable(&format!("Installing {:?}", mod_name));
                // For each file
                for entry in walkdir::WalkDir::new(mod_entry.path()) {
                    let file_entry = entry?;
                    if file_entry.file_type().is_file() {
                        let extracted_path = mod_entry.path().join(file_entry.file_name());
                        let install_path = self.install_dir().join(file_entry.file_name());
                        let mut extracted_file = File::open(extracted_path)?;
                        let mut install_file = File::create(install_path)?;
                        io::copy(&mut extracted_file, &mut install_file)?;
                    }
                }
                println!("Installed {:?} ", mod_name);
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
                // Load the archive
                let archive = ZipArchive::new(File::open(entry.path())?)?;
                // Delete if necessary
                if !(self.config.enabled.contains(&mod_name)
                    || self.config.disabled.contains(&mod_name))
                {
                    for name in archive.file_names() {
                        utils::remove_path(&install_dir, name)?;
                    }
                    utils::remove_path(extracted_dir, "")?;
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
