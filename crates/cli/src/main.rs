mod cmd;
mod flags;

use crate::cmd::init::{init_cmd, InitOpts};
use crate::cmd::start::{start, StartOpts};
use crate::flags::get_cli;
use clap::crate_version;
use colored::Colorize;

#[tokio::main]
async fn main() {
    let cli_matches = get_cli().get_matches();

    match cli_matches.subcommand() {
        Some(("start", sub_matches)) => {
            let ip = sub_matches.get_one::<String>("ip").cloned();
            let config_file = sub_matches.get_one::<String>("config").cloned().unwrap();
            let no_repl = sub_matches.get_one::<bool>("no-repl").cloned();

            let _ = start(StartOpts {
                ip,
                config_file,
                repl: no_repl.unwrap_or(false),
            })
            .await;
        }
        Some(("init", sub_matches)) => {
            let dir = sub_matches.get_one::<String>("directory").cloned();
            init_cmd(InitOpts { dir });
        }
        _ => {
            println!();
            println!("SJS {}", crate_version!());
            println!();
            println!("Run '{}' for help.", "schemajs --help".blue());
            println!();
            println!("Stuck? Join our Discord https://discord.gg/nRzTHygKn5");
            println!();
        }
    };
}
