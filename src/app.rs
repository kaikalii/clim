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
            help = "The path to the game's data folder, if it has one, relative to the game folder"
        )]
        data: Option<PathBuf>,
        #[structopt(
            long,
            short,
            help = "The path to the game's master plugins file, if it has one"
        )]
        plugins: Option<PathBuf>,
    },
    #[structopt(alias = "deploy", about = "Deploy mods")]
    Go,
    #[structopt(about = "Add mod archives to the active game")]
    Add {
        #[structopt(help = "Paths to the archive files")]
        archives: Vec<PathBuf>,
        #[structopt(
            long,
            short,
            help = "Whether to move the files instead of copying them"
        )]
        r#move: bool,
        #[structopt(long, short, help = "Enable all added mods")]
        enable: bool,
    },
    #[structopt(
        about = "Watch a directory for new downloads. \nNew downloads will be added to the active game's mods."
    )]
    Watch {
        #[structopt(help = "The folder to watch. Defaults to your user downloads folder")]
        folder: Option<PathBuf>,
        #[structopt(long, short, help = "Enable all added mods")]
        enable: bool,
    },
    #[structopt(about = "Enable mods")]
    Enable {
        #[structopt(help = "The names of the mods to enable. They do not need to be exact.")]
        names: Vec<String>,
        #[structopt(long, help = "Enable all mods")]
        all: bool,
    },
    #[structopt(about = "Disable mods")]
    Disable {
        #[structopt(help = "The names of the mods to disable. They do not need to be exact.")]
        names: Vec<String>,
        #[structopt(long, help = "Disable all mods")]
        all: bool,
    },
    #[structopt(about = "List all mods")]
    Mods,
    #[structopt(about = "List all enabled plugs")]
    Plugins,
    #[structopt(about = "Move a mod in the load order")]
    Move {
        #[structopt(help = "The name of the mod to move")]
        name: String,
        #[structopt(subcommand)]
        sub: MoveSubcommand,
    },
    #[structopt(about = "Uninstall mods")]
    Uninstall {
        #[structopt(help = "The names of the mods to uninstall")]
        names: Vec<String>,
        #[structopt(long, help = "Delete the archives as well")]
        delete_archives: bool,
        #[structopt(long, help = "Uninstall all mods")]
        all: bool,
    },
    #[structopt(about = "Set the active game")]
    SetActive {
        #[structopt(help = "The name of the game")]
        name: String,
    },
    #[structopt(about = "Get the name of the active game")]
    Active,
    #[structopt(about = "Open the active game's archives folder")]
    Archives,
    #[structopt(about = "Open the active game's main folder")]
    GameFolder,
}

#[derive(Debug, StructOpt)]
pub enum MoveSubcommand {
    #[structopt(about = "Move above another mod")]
    Above { name: String },
    #[structopt(about = "Move below another mod")]
    Below { name: String },
    #[structopt(about = "Move to the top")]
    Top,
    #[structopt(about = "Move to the bottom")]
    Bottom,
}
