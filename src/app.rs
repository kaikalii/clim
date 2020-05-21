use std::path::PathBuf;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about = "Command-line interface mod manager")]
pub enum App {
    #[structopt(about = "Add a game to climm")]
    Init {
        #[structopt(help = "The name of the game")]
        name: String,
        #[structopt(help = "The game's main folder")]
        game_folder: PathBuf,
        #[structopt(
            long,
            short,
            help = "The game's data folder, where mods should be placed, if it has one"
        )]
        data: Option<PathBuf>,
    },
    #[structopt(about = "Install and uninstall mods as defined by the active game's climm file")]
    Update,
    #[structopt(
        alias = "u",
        about = "Completely delete mods that are no longer present in the active game's climm file"
    )]
    Clean,
    #[structopt(about = "Edit the active game's climm fifle")]
    Edit {
        #[structopt(long, short, help = "Edit the global climm settings instead")]
        global: bool,
    },
    #[structopt(about = "Open the active game's downloads folder")]
    Downloads,
    #[structopt(about = "Open the active game's main folder")]
    GameFolder,
    #[structopt(about = "Set the active game")]
    SetActive {
        #[structopt(help = "The name of the game")]
        name: String,
    },
    #[structopt(about = "Get the name of the active game")]
    Active,
    #[structopt(
        about = "Watch a directory for new downloads. New downloads will be moved to the active game's downloads folder"
    )]
    Watch {
        #[structopt(help = "The folder to watch. Defaults to your user downloads folder")]
        folder: Option<PathBuf>,
    },
}
