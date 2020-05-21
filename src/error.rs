use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Toml error: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("Toml error: {0}")]
    Deserialize(#[from] toml::de::Error),
    #[error("Archive error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("No home directory")]
    NoHomeDirectory,
    #[error("Climm already manages {0}")]
    AlreadyManaged(String),
    #[error("Unknown game: {0}")]
    UnknownGame(String),
    #[error("No active game")]
    NoActiveGame,
}

pub type Result<T> = std::result::Result<T, Error>;
