use crate::context::context::SjsContext;
use crate::pool::pool_provider::SjsPoolProvider;
use crate::runtime::SchemeJsRuntime;
use r2d2::Pool;
use schemajs_helpers::helper::HelperCall;
use std::ops::DerefMut;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;

pub struct HelpersManager {
    pub rx_thread: JoinHandle<()>,
    pub ctx: Arc<SjsContext>,
}

impl HelpersManager {
    pub fn new(
        sjs_runtime_pool: Arc<Pool<SjsPoolProvider>>,
        rx: Receiver<HelperCall>,
        ctx: Arc<SjsContext>,
    ) -> HelpersManager {
        let handler_thread = Self::init(sjs_runtime_pool, rx, ctx.clone());

        Self {
            rx_thread: handler_thread,
            ctx,
        }
    }

    pub fn init(
        sjs_runtime_pool: Arc<Pool<SjsPoolProvider>>,
        mut rx: Receiver<HelperCall>,
        ctx: Arc<SjsContext>,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(cmd) = rx.recv().await {
                let runtime_pool = sjs_runtime_pool.clone();
                let ctx_clone = ctx.clone();
                let local = tokio::task::LocalSet::new();
                local.spawn_local(async move {
                    let mut rt = SchemeJsRuntime::new(ctx_clone.clone()).await.unwrap();
                    //let mut rt = runtime_pool.get().unwrap();
                    rt.acquire_lock().unwrap();
                    rt.call_helper(cmd).await;
                    println!("Hello world");
                });
            }
        })
    }
}
