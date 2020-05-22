#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Toml error: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("Toml error: {0}")]
    Deserialize(#[from] toml::de::Error),
    #[error("No home directory")]
    NoHomeDirectory,
    #[error("No user downloads folder")]
    NoDownloadsDirectory,
    #[error("Climm already manages {0}")]
    AlreadyManaged(String),
    #[error("Unknown game: {0}")]
    UnknownGame(String),
    #[error("No active game")]
    NoActiveGame,
    #[error("Directory walk error: {0}")]
    WalkDir(#[from] walkdir::Error),
    #[error("Fomod error: {0}")]
    Fomod(#[from] crate::fomod::Error),
    #[error("No mod found for {0:?}")]
    UnknownMod(String),
    #[error("Notify error: {0}")]
    Notify(#[from] notify::Error),
    #[error("Cannot move {0} in relation to itself")]
    SelfRelativeMove(String),
}

pub type Result<T> = std::result::Result<T, Error>;
