use crate::context::context::SjsContext;
use crate::pool::pool_provider::SjsPoolProvider;
use crate::runtime::SchemeJsRuntime;
use crate::thread::WORKER_RT;
use r2d2::Pool;
use schemajs_helpers::helper::HelperCall;
use std::ops::DerefMut;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;

pub struct HelpersManager {
    pub ctx: Arc<SjsContext>,
}

impl HelpersManager {
    pub fn new(
        sjs_runtime_pool: Arc<Pool<SjsPoolProvider>>,
        rx: Receiver<HelperCall>,
        ctx: Arc<SjsContext>,
    ) -> HelpersManager {
        let handler_thread = Self::init(sjs_runtime_pool, rx, ctx.clone());

        Self { ctx }
    }

    pub fn init(
        sjs_runtime_pool: Arc<Pool<SjsPoolProvider>>,
        mut rx: Receiver<HelperCall>,
        ctx: Arc<SjsContext>,
    ) {
        let rt = &WORKER_RT;
        rt.spawn_pinned(move || {
            tokio::task::spawn_local(async move {
                while let Some(cmd) = rx.recv().await {
                    let permit = SchemeJsRuntime::acquire().await;
                    match SchemeJsRuntime::new(ctx.clone()).await {
                        Ok(rt) => {
                            let lock = rt.acquire_lock(); // TODO: Wait for lock
                            match lock {
                                Ok(_) => {
                                    let mut runtime = scopeguard::guard(rt, |mut runtime| unsafe {
                                        runtime.js_runtime.v8_isolate().enter();
                                        runtime.release_lock();
                                    });

                                    runtime.call_helper(cmd).await;

                                    unsafe {
                                        runtime.js_runtime.v8_isolate().exit();
                                    }

                                    drop(permit);
                                }
                                Err(_) => {}
                            }
                        }
                        Err(_) => {}
                    }
                }
            })
        });
    }
}
