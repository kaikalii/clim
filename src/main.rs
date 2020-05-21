mod app;
mod config;
mod error;
mod library;
mod utils;
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

    let app = App::from_args();

    let mut gc = GlobalConfig::open()?;

    match app {
        App::Init {
            name,
            game_folder,
            data,
        } => {
            gc.init_game(name, game_folder, data)?;
        }
        App::Update => {
            let mut game = gc.active_game()?;
            game.update()?;
        }
    }

    Ok(())
}
