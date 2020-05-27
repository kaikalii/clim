use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub trait AndCreateDirs: Sized {
    fn and_create_dirs<E>(self) -> Result<Self, E>
    where
        E: From<io::Error>;
}

impl<P> AndCreateDirs for P
where
    P: AsRef<Path>,
{
    fn and_create_dirs<E>(self) -> Result<Self, E>
    where
        E: From<io::Error>,
    {
        fs::create_dir_all(&self)?;
        Ok(self)
    }
}

pub fn clim_dir() -> crate::Result<PathBuf> {
    dirs::home_dir()
        .ok_or(crate::Error::NoHomeDirectory)
        .and_then(|home| home.join(".clim").and_create_dirs())
}

pub fn global_config() -> crate::Result<PathBuf> {
    clim_dir().map(|clim| clim.join("config.toml"))
}

pub fn game_dir(game: &str) -> crate::Result<PathBuf> {
    clim_dir().and_then(|clim| clim.join(game).and_create_dirs())
}

pub fn archives_dir(game: &str) -> crate::Result<PathBuf> {
    game_dir(game).and_then(|game| game.join("archives").and_create_dirs())
}

pub fn extracted_dir(game: &str, mod_name: &str) -> crate::Result<PathBuf> {
    game_dir(game).and_then(|game| game.join("extracted").join(mod_name).and_create_dirs())
}
