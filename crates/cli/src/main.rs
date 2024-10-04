mod flags;

use crate::flags::get_cli;
use anyhow::Error;
use base::context::context::SjsContext;
use base::runner::{SjsRunner, SjsRunnerConfig};
use base::runtime::SchemeJsRuntime;
use clap::ArgMatches;
use schemajs_grpc::server::{GrpcServer, GrpcServerArgs};
use std::path::PathBuf;
use std::sync::Arc;

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
                let runner = SjsRunner::new(SjsRunnerConfig {
                    max_helper_processing_capacity: 100,
                    max_runtimes: 10,
                    config_path: PathBuf::from(config_file),
                    data_path: None,
                });

                {
                    // Loader runtime
                    let rt = SchemeJsRuntime::new(runner.sjs_context.clone())
                        .await
                        .unwrap();
                    drop(rt);
                }

                let internal_manager = runner.sjs_context.internal_manager.clone();

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
    });
}
