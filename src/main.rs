mod app;
mod error;
mod fomod;
mod game;
mod library;
mod utils;
use app::*;

use std::{
    fs,
    io::{stdin, BufRead},
};

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
        App::SetActive { name } => {
            if gc.games.contains(&name) {
                println!("Set {:?} as active game", name);
                gc.active_game = Some(name);
            } else {
                return Err(Error::UnknownGame(name));
            }
        }
        App::Active => {
            if let Some(name) = &gc.active_game {
                println!("{:?} is currently active", name);
            } else {
                println!("No active game");
            }
        }
        App::Watch { folder } => {
            use notify::{
                event::CreateKind, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
            };
            let path = if let Some(folder) = folder {
                folder
            } else {
                dirs::download_dir().ok_or(Error::NoDownloadsDirectory)?
            };
            let game_downloads_dir = library::downloads_dir(&gc.active_game()?.name)?;
            let mut watcher: RecommendedWatcher = Watcher::new_immediate(move |res| match res {
                Ok(Event {
                    kind: EventKind::Create(CreateKind::Any),
                    paths,
                    ..
                })
                | Ok(Event {
                    kind: EventKind::Create(CreateKind::File),
                    paths,
                    ..
                }) => {
                    for path in paths {
                        if path.extension().map_or(false, |ext| ext != "crdownload") {
                            if let Err(e) = fs::rename(
                                &path,
                                game_downloads_dir.join(path.file_name().unwrap()),
                            ) {
                                println!("{}", e);
                            } else {
                                println!("Added mod {:?}", path.file_name().unwrap());
                            }
                        }
                    }
                }
                _ => {}
            })?;
            watcher.watch(&path, RecursiveMode::NonRecursive)?;
            println!("Watching {:?}. Press enter to end...", path);
            stdin().lock().lines().next().unwrap()?;
        }
    }

    Ok(())
}
