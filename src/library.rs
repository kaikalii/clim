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

pub fn climm_dir() -> crate::Result<PathBuf> {
    dirs::home_dir()
        .ok_or(crate::Error::NoHomeDirectory)
        .and_then(|home| home.join(".climm").and_create_dirs())
}

pub fn global_config() -> crate::Result<PathBuf> {
    climm_dir().map(|climm| climm.join("config.toml"))
}

pub fn game_dir(game: &str) -> crate::Result<PathBuf> {
    climm_dir().and_then(|climm| climm.join(game).and_create_dirs())
}
