use clap::{arg, crate_version, ArgAction, Command};

pub(super) fn get_cli() -> Command {
    Command::new(env!("CARGO_BIN_NAME"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .version(format!("SJS {}", crate_version!()))
        .subcommand(get_start_command())
}

fn get_start_command() -> Command {
    Command::new("start")
        .about("Start a new SJS server")
        .arg(
            arg!(-i --ip <HOST>)
                .help("Host IP address to listen on")
                .default_value("[::1]:34244")
                .env("SJS_HOST"),
        )
        .arg(
            arg!(-c --config <HOST>)
                .help("Path to SchemaJS.toml or directory containing it")
                .default_value("./")
                .env("SJS_CONFIG"),
        )
        .arg(
            arg!(--"no-repl")
                .help("Whether it should initialize the REPL when running")
                .action(ArgAction::SetTrue),
        )
}
