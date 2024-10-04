use crate::context::context::SjsContext;
use crate::pool::pool_provider::SjsPoolProvider;
use r2d2::Pool;
use std::sync::Arc;

pub mod pool_provider;

pub struct SjsRuntimePool {
    pub pool: Arc<Pool<SjsPoolProvider>>,
}

impl SjsRuntimePool {
    pub fn new(shared_context: Arc<SjsContext>, max_runtimes: u32) -> Self {
        let provider = SjsPoolProvider { shared_context };

        let a = "";

        let pool = r2d2::Pool::builder()
            .max_size(max_runtimes)
            .min_idle(Some(0))
            .build(provider)
            .unwrap();

        let b = "";

        Self {
            pool: Arc::new(pool),
        }
    }
}
