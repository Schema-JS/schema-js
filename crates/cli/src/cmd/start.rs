use crate::cmd::repl::repl;
use base::runner::{SjsRunner, SjsRunnerConfig};
use base::runtime::SchemeJsRuntime;
use clap::crate_version;
use colored::Colorize;
use schemajs_grpc::server::{GrpcServer, GrpcServerArgs};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub(crate) struct StartOpts {
    pub ip: Option<String>,
    pub config_file: String,
    pub repl: bool,
}

async fn start_server(ip: Option<String>, runner: Arc<SjsRunner>) {
    let ip = ip.unwrap_or_else(|| runner.sjs_context.config.grpc.host.clone());
    let grpc_server = GrpcServer::new(GrpcServerArgs {
        db_manager: runner.sjs_context.internal_manager.clone(),
        ip: Some(ip.clone()),
    });

    println!();
    println!("SJS {}", crate_version!());
    println!("Exit using ctrl+c");
    println!();

    println!("> {}", format!("Starting GRPC at {}", ip).blue());

    grpc_server.start().await.unwrap();
}

pub(crate) async fn start(opts: StartOpts) {
    let StartOpts {
        config_file,
        ip,
        repl: no_repl,
    } = opts;

    let runner = SjsRunner::new(SjsRunnerConfig {
        max_helper_processing_capacity: 100,
        max_runtimes: 10,
        config_path: PathBuf::from(config_file),
        data_path: None,
    });

    let arc_runner = Arc::new(runner);

    {
        // Loader runtime
        let rt = SchemeJsRuntime::new(arc_runner.sjs_context.clone())
            .await
            .unwrap();
        drop(rt);
    }

    let runner = arc_runner.clone();
    tokio::spawn(async move { start_server(ip, runner).await });
    tokio::time::sleep(Duration::from_secs(1)).await;

    if !no_repl {
        let _repl = repl(arc_runner).await;
    } else {
        loop {}
    }
}
