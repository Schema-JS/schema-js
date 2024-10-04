use once_cell::sync::Lazy;
use std::num::NonZeroUsize;

pub const DEFAULT_USER_WORKER_POOL_SIZE: usize = 1;

pub static WORKER_RT: Lazy<tokio_util::task::LocalPoolHandle> = Lazy::new(|| {
    let maybe_pool_size = std::env::var("SJS_WORKER_POOL_SIZE")
        .ok()
        .and_then(|it| it.parse::<usize>().ok())
        .map(|it| {
            if it < DEFAULT_USER_WORKER_POOL_SIZE {
                DEFAULT_USER_WORKER_POOL_SIZE
            } else {
                it
            }
        });

    tokio_util::task::LocalPoolHandle::new(if cfg!(debug_assertions) {
        maybe_pool_size.unwrap_or(DEFAULT_USER_WORKER_POOL_SIZE)
    } else {
        maybe_pool_size.unwrap_or(
            std::thread::available_parallelism()
                .ok()
                .map(NonZeroUsize::get)
                .unwrap_or(DEFAULT_USER_WORKER_POOL_SIZE),
        )
    })
});
