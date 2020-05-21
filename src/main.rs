mod app;
mod error;
mod fomod;
mod game;
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
    use game::*;

    let app = App::from_args();

    let mut gc = GlobalConfig::open()?;

    match app {
        App::Init {
            name,
            game_folder,
            data,
        } => {
            gc.init_game(name, game_folder, data, None)?;
        }
        App::Update => gc.active_game()?.update()?,
        App::Clean => gc.active_game()?.clean()?,
        App::Edit { global } => {
            open::that(if global {
                library::global_config()?
            } else {
                gc.active_game()?.config_file()?
            })?;
        }
        App::Downloads => {
            open::that(library::downloads_dir(&gc.active_game()?.name)?)?;
        }
        App::GameFolder => {
            open::that(&gc.active_game()?.config.game_folder)?;
        }
    }

    Ok(())
}
