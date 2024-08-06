use anyhow::Error;
use base::runtime::WorkerContextInitOpts;
use std::path::PathBuf;

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .thread_name("sjs-main")
        .build()
        .unwrap();

    let local = tokio::task::LocalSet::new();
    let res: Result<(), Error> = local.block_on(&runtime, async {
        let mut rt = base::runtime::SchemeJsRuntime::new(WorkerContextInitOpts {
            config_path: PathBuf::from("/Users/andrespirela/Documents/workspace/pirela/schema-js/crates/base/test_cases/default-db/SchemeJS.toml"),
            data_path: None,
        }).await.unwrap();

        Ok(())
    });
}
