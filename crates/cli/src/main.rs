mod flags;

use crate::flags::get_cli;
use anyhow::Error;
use base::runtime::WorkerContextInitOpts;
use clap::ArgMatches;
use schemajs_grpc::server::{GrpcServer, GrpcServerArgs};
use std::path::PathBuf;

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .thread_name("sjs-main")
        .build()
        .unwrap();

    let local = tokio::task::LocalSet::new();
    let res: Result<(), Error> = local.block_on(&runtime, async {
        let cli_matches = get_cli().get_matches();

        match cli_matches.subcommand() {
            Some(("start", sub_matches)) => {
                let ip = sub_matches.get_one::<String>("ip").cloned().unwrap();
                let config_file = sub_matches.get_one::<String>("config").cloned().unwrap();

                let mut rt = base::runtime::SchemeJsRuntime::new(WorkerContextInitOpts {
                    config_path: PathBuf::from(config_file),
                    data_path: None,
                })
                .await
                .unwrap();
                let internal_manager = rt.internal_manager.clone();

                let grpc_server = GrpcServer::new(GrpcServerArgs {
                    db_manager: internal_manager,
                    ip: Some(ip.clone()),
                });

                println!("Starting GRPC at {}", ip);

                grpc_server.start().await.unwrap();
            }
            _ => {}
        };

        Ok(())

        // let mut rt = base::runtime::SchemeJsRuntime::new(WorkerContextInitOpts {
        //     config_path: PathBuf::from("/Users/andrespirela/Documents/workspace/pirela/schema-js/crates/base/test_cases/default-db/SchemeJS.toml"),
        //     data_path: None,
        // }).await.unwrap();
        //
        // Ok(())
    });
}
