mod cmd;
mod flags;

use crate::cmd::start::{start, StartOpts};
use crate::flags::get_cli;

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
        _ => {}
    };
}
