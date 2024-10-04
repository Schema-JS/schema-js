use crate::context::context::SjsContext;
use crate::helpers::HelpersManager;
use crate::pool::SjsRuntimePool;
use schemajs_helpers::create_helper_channel;
use schemajs_helpers::helper::HelperCall;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

pub struct SjsRunner {
    pub sjs_context: Arc<SjsContext>,
    pub rt_pool: Arc<SjsRuntimePool>,
    pub helpers_manager: HelpersManager,
    pub helper_tx: Sender<HelperCall>,
}

pub struct SjsRunnerConfig {
    pub max_helper_processing_capacity: usize,
    pub max_runtimes: u32,
    pub config_path: PathBuf,
    pub data_path: Option<PathBuf>,
}

impl SjsRunner {
    pub fn new(config: SjsRunnerConfig) -> Self {
        let (helper_tx, helper_rx) = create_helper_channel(config.max_helper_processing_capacity);
        let context = Arc::new(
            SjsContext::new(config.config_path, config.data_path, helper_tx.clone()).unwrap(),
        );
        let rt_pool = Arc::new(SjsRuntimePool::new(context.clone(), config.max_runtimes));
        let helpers_manager = HelpersManager::new(rt_pool.pool.clone(), helper_rx, context.clone());

        Self {
            helper_tx,
            sjs_context: context,
            rt_pool,
            helpers_manager,
        }
    }
}

#[cfg(test)]
mod runner_tests {
    use crate::runner::{SjsRunner, SjsRunnerConfig};
    use schemajs_helpers::helper::HelperCall;
    use std::path::PathBuf;
    use std::time::Duration;

    #[tokio::test]
    pub async fn test_runner_with_helpers() {
        println!("Runner created");
        let runner = SjsRunner::new(SjsRunnerConfig {
            max_helper_processing_capacity: 10,
            max_runtimes: 3,
            config_path: PathBuf::from("./test_cases/default-db"),
            data_path: None,
        });

        println!("Before tx created");

        runner
            .helper_tx
            .send(HelperCall::CustomQuery {
                table: "users".to_string(),
                identifier: "helloWorld".to_string(),
                req: serde_json::Value::String("Hello World".to_string()),
            })
            .await
            .unwrap();

        println!("After tx created");

        tokio::time::sleep(Duration::from_secs(10)).await;

        runner.helpers_manager.rx_thread.abort();
    }
}
