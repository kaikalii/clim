use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use indexmap::IndexSet;
use serde_derive::{Deserialize, Serialize};
use zip::ZipArchive;

use crate::library;

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
        let bytes = toml::to_vec(self)?;
        fs::write(library::global_config()?, &bytes).map_err(Into::into)
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
                enabled: IndexSet::new(),
                disabled: HashSet::new(),
            },
        }
        .save()?;
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
    pub fn open(name: &str) -> crate::Result<Self> {
        let bytes = fs::read(game_config_file(name)?)?;
        let config: Config = toml::from_slice(&bytes)?;
        Ok(Game {
            name: name.into(),
            config,
        })
    }
    pub fn save(&self) -> crate::Result<()> {
        let bytes = toml::to_vec(&self.config)?;
        fs::write(self.config_file()?, &bytes).map_err(Into::into)
    }
    // pub fn update(&mut self) -> crate::Result<()> {
    //     for entry in fs::read_dir(&self.path)? {}
    //     Ok(())
    // }
}

impl Drop for Game {
    fn drop(&mut self) {
        if let Err(e) = self.save() {
            println!("Error saving config: {}", e);
        }
    }
}
