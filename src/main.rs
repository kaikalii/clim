mod utils;

mod app;
mod error;
mod fomod;
mod game;
mod library;
use app::*;

use std::io::{stdin, BufRead};

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
            plugins,
        } => {
            gc.init_game(name, game_folder, data, plugins)?;
        }
        App::Go => gc.active_game()?.go()?,
        App::Add {
            archives,
            r#move,
            enable,
        } => gc.active_game()?.add(&archives, r#move, enable)?,
        App::Enable { names, all } => {
            let mut game = gc.active_game()?;
            if all {
                game.enable_all()?;
            } else {
                for name in names {
                    game.enable(&name)?;
                }
            }
        }
        App::Disable { names, all } => {
            let mut game = gc.active_game()?;
            if all {
                game.disable_all()?;
            } else {
                for name in names {
                    game.disable(&name)?;
                }
            }
        }
        App::Mods => {
            for (mod_name, mm) in &gc.active_game()?.config.mods {
                if mm.enabled {
                    colorln!(normal, "{}", mod_name);
                } else {
                    colorln!(dimmed, "{}", mod_name);
                }
            }
        }
        App::Plugins => {
            for plugin in gc.active_game()?.plugins() {
                println!("{}", plugin.to_string_lossy());
            }
        }
        App::Move { name, sub } => gc.active_game()?.move_mod(name, sub)?,
        App::Uninstall {
            names,
            delete_archives,
            all,
        } => {
            let mut game = gc.active_game()?;
            if all {
                game.uninstall_all(delete_archives)?;
            } else {
                for name in names {
                    game.uninstall(&name, delete_archives)?;
                }
            }
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
        App::Downloads => {
            open::that(library::downloads_dir(&gc.active_game()?.name)?)?;
        }
        App::GameFolder => {
            open::that(&gc.active_game()?.config.game_folder)?;
        }
        App::Watch { folder, enable } => {
            use notify::{
                event::CreateKind, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
            };
            let path = if let Some(folder) = folder {
                folder
            } else {
                dirs::download_dir().ok_or(Error::NoDownloadsDirectory)?
            };
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
                            if let Err(e) = gc
                                .active_game()
                                .and_then(|mut game| game.add(&[path], true, enable))
                            {
                                println!("{}", e);
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
