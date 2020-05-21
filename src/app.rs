use std::path::PathBuf;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub enum App {
    Init {
        name: String,
        game_folder: PathBuf,
        #[structopt(long, short)]
        data: Option<PathBuf>,
    },
    Update,
    Clean,
    Edit {
        #[structopt(long, short)]
        global: bool,
    },
    Downloads,
    GameFolder,
}
