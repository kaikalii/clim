use std::{
    collections::HashSet,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use indexmap::IndexMap;
use itertools::Itertools;
use pathdiff::diff_paths;
use serde_derive::{Deserialize, Serialize};
use walkdir::{DirEntry, WalkDir};

use crate::{
    app::MoveSubcommand,
    colorln, fomod,
    library::{self},
    utils, waitln,
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
        plugins: Option<PathBuf>,
        exe: Option<PathBuf>,
    ) -> crate::Result<()> {
        if self.games.contains(&name) {
            return Err(crate::Error::AlreadyManaged(name));
        }
        self.active_game = Some(name.clone());
        self.games.insert(name.clone());
        Game {
            name: name.clone(),
            config: Config {
                data_folder: data,
                game_folder: folder,
                plugins_file: plugins,
                exe,
                deployment: DeploymentMethod::default(),
                mods: IndexMap::new(),
            },
        }
        .save()?;
        library::archives_dir(&name)?;
        println!("clim initialized {}", name);
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

fn _true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ManagedMod {
    pub enabled: bool,
    pub extracted: Option<PathBuf>,
    pub archive: PathBuf,
    pub parts: Vec<PathBuf>,
}

impl ManagedMod {
    pub fn new(archive: PathBuf) -> Self {
        ManagedMod {
            archive,
            ..Self::default()
        }
    }
    pub fn part_paths(&self) -> Vec<PathBuf> {
        if self.parts.is_empty() {
            if let Some(extr) = &self.extracted {
                vec![extr.clone()]
            } else {
                Vec::new()
            }
        } else {
            self.parts.clone()
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DeploymentMethod {
    Hardlink,
    Symlink,
}

impl Default for DeploymentMethod {
    fn default() -> Self {
        DeploymentMethod::Hardlink
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub game_folder: PathBuf,
    pub data_folder: Option<PathBuf>,
    pub plugins_file: Option<PathBuf>,
    pub exe: Option<PathBuf>,
    pub deployment: DeploymentMethod,
    pub mods: IndexMap<String, ManagedMod>,
}

fn install_dir(
    game_folder: &Path,
    data_folder: Option<&Path>,
    mod_has_data_folder: bool,
) -> PathBuf {
    if let (Some(data), false) = (&data_folder, mod_has_data_folder) {
        game_folder.join(data)
    } else {
        game_folder.to_path_buf()
    }
}

fn get_mod<'a>(
    mods: &'a mut IndexMap<String, ManagedMod>,
    name: &str,
) -> crate::Result<(&'a str, &'a mut ManagedMod)> {
    let name = name.to_lowercase();
    mods.iter_mut()
        .find(|(mod_name, _)| mod_name.to_lowercase().contains(&name))
        .map(|(mod_name, mm)| (mod_name.as_str(), mm))
        .ok_or(crate::Error::UnknownMod(name))
}

impl Config {
    pub fn get_mod(&mut self, name: &str) -> crate::Result<(&str, &mut ManagedMod)> {
        get_mod(&mut self.mods, name)
    }
}

pub struct Game {
    pub name: String,
    pub config: Config,
}

const GAME_CONFIG_FILE: &str = "clim.toml";

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
        let string = toml::to_string_pretty(&self.config)?;
        fs::write(self.config_file()?, &string).map_err(Into::into)
    }
    pub fn get_mod(&mut self, name: &str) -> crate::Result<(&str, &mut ManagedMod)> {
        self.config.get_mod(name)
    }
    pub fn add(&mut self, paths: &[PathBuf], mv: bool, enable: bool) -> crate::Result<()> {
        for path in paths {
            if let Some(file_name) = path.file_name() {
                let download_copy = library::archives_dir(&self.name)?.join(file_name);
                if mv {
                    fs::rename(path, &download_copy)?;
                } else {
                    fs::copy(path, &download_copy)?;
                }
                let mod_name = path.file_stem().unwrap().to_string_lossy().into_owned();
                self.config
                    .mods
                    .entry(mod_name.clone())
                    .or_insert_with(|| {
                        println!("Added {:?}", mod_name);
                        ManagedMod::new(download_copy)
                    })
                    .enabled = enable;
            }
        }
        Ok(())
    }
    fn enable_mod(
        data_folder: Option<&Path>,
        mod_name: &str,
        mm: &mut ManagedMod,
    ) -> crate::Result<()> {
        Game::extract_mod(mod_name, data_folder, mod_name, mm)?;
        if !mm.enabled {
            mm.enabled = true;
            println!("Enabled {}", mod_name);
        }
        Ok(())
    }
    pub fn enable(&mut self, name: &str) -> crate::Result<()> {
        let (mod_name, mm) = get_mod(&mut self.config.mods, name)?;
        Game::enable_mod(self.config.data_folder.as_deref(), mod_name, mm)
    }
    pub fn enable_all(&mut self) -> crate::Result<()> {
        for (mod_name, mm) in &mut self.config.mods {
            Game::enable_mod(self.config.data_folder.as_deref(), mod_name, mm)?;
        }
        Ok(())
    }
    fn disable_mod(mod_name: &str, mm: &mut ManagedMod) {
        if mm.enabled {
            mm.enabled = false;
            println!("Disabled {}", mod_name);
        }
    }
    pub fn disable(&mut self, name: &str) -> crate::Result<()> {
        let (mod_name, mm) = get_mod(&mut self.config.mods, name)?;
        Game::disable_mod(mod_name, mm);
        Ok(())
    }
    pub fn disable_all(&mut self) -> crate::Result<()> {
        for (mod_name, mm) in &mut self.config.mods {
            Game::disable_mod(mod_name, mm);
        }
        Ok(())
    }
    fn extract(&mut self) -> crate::Result<()> {
        for (mod_name, mm) in &mut self.config.mods {
            Game::extract_mod(&self.name, self.config.data_folder.as_deref(), mod_name, mm)?;
        }
        Ok(())
    }
    fn extract_mod(
        game_name: &str,
        data_folder: Option<&Path>,
        mod_name: &str,
        mm: &mut ManagedMod,
    ) -> crate::Result<()> {
        if mm.enabled && mm.extracted.is_none() {
            waitln!("Extracting {:?}...", mod_name);
            let extracted_dir = library::extracted_dir(game_name, mod_name)?;
            let _ = fs::remove_dir_all(&extracted_dir);
            // Extract
            let status = Command::new("7z")
                .arg("x")
                .arg(&mm.archive)
                .arg(format!("-o{}", extracted_dir.to_string_lossy()))
                .arg("-spe")
                .output()?
                .status;
            if !status.success() {
                return Err(crate::Error::Extraction {
                    archive: mm.archive.clone(),
                    code: status.code(),
                });
            }
            // If there is exactly one entry in the folder and it is not a Data folder
            if fs::read_dir(&extracted_dir)?
                .filter_map(Result::ok)
                .filter(|entry| entry.path().is_dir())
                .count()
                == 1
                && !contains_data_folder(&extracted_dir, data_folder)?
            {
                // Get the inner folder
                let narrowed = fs::read_dir(&extracted_dir)?
                    .filter_map(Result::ok)
                    .find(|entry| entry.path().is_dir())
                    .unwrap()
                    .path();
                // Rename all entries in the inner folder to be in the outer folder
                for entry in fs::read_dir(&narrowed)?.filter_map(Result::ok) {
                    let path_diff = diff_paths(entry.path(), &narrowed).unwrap();
                    let new_path = extracted_dir.join(path_diff);
                    fs::rename(entry.path(), new_path)?;
                }
                // Remove the now-empty inner folder
                fs::remove_dir(narrowed)?;
            }
            // Capitalize all folders on unix
            if cfg!(unix) {
                for entry in WalkDir::new(&extracted_dir)
                    .into_iter()
                    .filter_map(Result::ok)
                {
                    if entry.file_type().is_dir() {
                        let _ = fs::rename(
                            entry.path(),
                            utils::capitalize_path(&extracted_dir, entry.path()),
                        );
                    }
                }
            }
            mm.extracted = Some(extracted_dir);
            colorln!(green, "done");
        }
        Ok(())
    }
    fn undeploy_mod(
        game_folder: &Path,
        data_folder: Option<&Path>,
        mm: &mut ManagedMod,
    ) -> crate::Result<()> {
        for install_src in mm.part_paths() {
            let contains_data_folder = match contains_data_folder(&install_src, data_folder) {
                Ok(cdf) => cdf,
                Err(_) => continue,
            };
            let install_dir = install_dir(&game_folder, data_folder, contains_data_folder);
            let src_diff = differ(&install_src);
            for entry in WalkDir::new(&install_src) {
                let file_entry = entry?;
                utils::remove_path(&install_dir, src_diff(&file_entry.path()).unwrap())?;
            }
        }
        Ok(())
    }
    fn undeploy(&mut self) -> crate::Result<()> {
        for (_, mm) in &mut self.config.mods {
            Game::undeploy_mod(
                &self.config.game_folder,
                self.config.data_folder.as_deref(),
                mm,
            )?;
        }
        Ok(())
    }
    fn deploy(&mut self) -> crate::Result<()> {
        for (mod_name, mm) in &mut self.config.mods {
            if let (Some(extracted_dir), true) = (&mm.extracted, mm.enabled) {
                // Search for a Fomod config
                let config = WalkDir::new(&extracted_dir)
                    .into_iter()
                    .filter_map(Result::ok)
                    .find(|entry| {
                        entry
                            .path()
                            .file_name()
                            .map_or(false, |name| name == "ModuleConfig.xml")
                    })
                    .map(DirEntry::into_path);
                // Get a list of folders from which to install things
                let install_folders = if !mm.parts.is_empty() {
                    mm.parts.clone()
                } else if config.is_some() {
                    let paths = fomod::pseudo_fomod(mod_name, &extracted_dir)?;
                    mm.parts = paths.clone();
                    paths
                } else {
                    vec![extracted_dir.clone()]
                };
                // For each folder
                for install_src in install_folders {
                    let contains_data_folder =
                        contains_data_folder(&install_src, self.config.data_folder.as_deref())?;
                    let install_dir = install_dir(
                        &self.config.game_folder,
                        self.config.data_folder.as_deref(),
                        contains_data_folder,
                    );
                    let src_diff = differ(&install_src);
                    // For each file
                    for entry in WalkDir::new(&install_src) {
                        let file_entry = entry?;
                        if file_entry.file_type().is_file() {
                            let extracted_path =
                                install_src.join(src_diff(&file_entry.path()).unwrap());
                            let install_path =
                                install_dir.join(src_diff(&file_entry.path()).unwrap());
                            utils::create_dirs(&install_path)?;
                            // Deploy
                            match self.config.deployment {
                                DeploymentMethod::Hardlink => {
                                    let _ = fs::hard_link(extracted_path, install_path);
                                }
                                DeploymentMethod::Symlink => {
                                    #[cfg(unix)]
                                    let _ =
                                        std::os::unix::fs::symlink(extracted_path, install_path);
                                    #[cfg(windows)]
                                    let _ = std::os::windows::fs::hardlink(
                                        extracted_path,
                                        install_path,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    pub fn plugins(&self) -> impl Iterator<Item = PathBuf> + '_ {
        self.config
            .mods
            .values()
            .filter(|mm| mm.enabled)
            .flat_map(|mm| mm.part_paths())
            .flat_map(|path| WalkDir::new(path).into_iter().filter_map(Result::ok))
            .filter_map(|entry| {
                entry.path().extension().and_then(|ext| {
                    if ["esp", "esm", "esl"].contains(&ext.to_string_lossy().as_ref()) {
                        Some(entry.path().file_name().unwrap().into())
                    } else {
                        None
                    }
                })
            })
            .dedup()
    }
    pub fn write_plugins(&self) -> crate::Result<()> {
        if let Some(plugins) = &self.config.plugins_file {
            let mut file = File::create(plugins)?;
            for plugin in self.plugins() {
                writeln!(file, "*{}", plugin.to_string_lossy())?;
            }
        }
        Ok(())
    }
    pub fn go(&mut self) -> crate::Result<()> {
        self.extract()?;
        waitln!("Deploying...");
        self.undeploy()?;
        self.deploy()?;
        self.write_plugins()?;
        colorln!(green, "done");
        Ok(())
    }
    fn uninstall_mod(
        game_folder: &Path,
        data_folder: Option<&Path>,
        mod_name: &str,
        mm: &mut ManagedMod,
        delete_archives: bool,
    ) -> crate::Result<()> {
        Game::disable_mod(mod_name, mm);
        Game::undeploy_mod(game_folder, data_folder, mm)?;
        if delete_archives {
            fs::remove_file(&mm.archive)?;
        }
        if let Some(extracted) = mm.extracted.take() {
            fs::remove_dir_all(extracted)?;
            println!("Uninstalled {}", mod_name);
        }
        Ok(())
    }
    pub fn uninstall(&mut self, name: &str, delete_archives: bool) -> crate::Result<()> {
        let (mod_name, mm) = get_mod(&mut self.config.mods, name)?;
        Game::uninstall_mod(
            &self.config.game_folder,
            self.config.data_folder.as_deref(),
            mod_name,
            mm,
            delete_archives,
        )?;
        if delete_archives {
            let mod_name = mod_name.to_string();
            self.config.mods.remove(&mod_name);
        }
        Ok(())
    }
    pub fn uninstall_all(&mut self, delete_archives: bool) -> crate::Result<()> {
        for (mod_name, mm) in &mut self.config.mods {
            Game::uninstall_mod(
                &self.config.game_folder,
                self.config.data_folder.as_deref(),
                mod_name,
                mm,
                delete_archives,
            )?;
        }
        if delete_archives {
            self.config.mods.clear();
        }
        Ok(())
    }
    pub fn move_mod(&mut self, moved: String, to: MoveSubcommand) -> crate::Result<()> {
        let moved_name = self.get_mod(&moved)?.0.to_string();
        macro_rules! relative {
            ($other:expr, $add:expr) => {{
                let other_name = self.get_mod(&$other)?.0.to_string();
                if moved_name == other_name {
                    return Err(crate::Error::SelfRelativeMove(moved_name));
                }
                let moved_mm = self.config.mods.shift_remove(&moved_name).unwrap();
                let other_index = self
                    .config
                    .mods
                    .keys()
                    .position(|mod_name| mod_name == &other_name)
                    .unwrap();
                let mut mods_drain = self.config.mods.drain(..);
                let mut new_mods: IndexMap<_, _> =
                    mods_drain.by_ref().take(other_index + $add).collect();
                new_mods.insert(moved_name, moved_mm);
                new_mods.extend(mods_drain);
                self.config.mods = new_mods;
            }};
        }
        match to {
            MoveSubcommand::Above { name: other } => relative!(other, 0),
            MoveSubcommand::Below { name: other } => relative!(other, 1),
            MoveSubcommand::Top => {
                let moved_mm = self.config.mods.shift_remove(&moved_name).unwrap();
                let mut new_mods = IndexMap::new();
                new_mods.insert(moved_name, moved_mm);
                new_mods.extend(self.config.mods.drain(..));
                self.config.mods = new_mods;
            }
            MoveSubcommand::Bottom => {
                let moved_mm = self.config.mods.shift_remove(&moved_name).unwrap();
                self.config.mods.insert(moved_name, moved_mm);
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

fn differ<P>(top: &P) -> impl Fn(&'_ Path) -> Option<PathBuf> + '_
where
    P: AsRef<Path>,
{
    move |path| diff_paths(path, top)
}

fn contains_data_folder(path: &Path, data_folder: Option<&Path>) -> crate::Result<bool> {
    Ok(if let Some(data) = data_folder {
        fs::read_dir(&path)?
            .filter_map(Result::ok)
            .any(|entry| entry.path().ends_with(data))
    } else {
        false
    })
}
