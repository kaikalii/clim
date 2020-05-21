mod app;
mod config;
mod error;
mod library;
use app::*;

use structopt::StructOpt;

use error::{Error, Result};

fn main() {
    if let Err(e) = run() {
        println!("{}", e);
    }
}

fn run() -> Result<()> {
    use config::*;

    match App::from_args() {
        App::Init {
            name,
            game_folder,
            data,
        } => {
            let mut gc = GlobalConfig::open()?;
            gc.init_game(name, game_folder, data)?;
        }
    }

    Ok(())
}
