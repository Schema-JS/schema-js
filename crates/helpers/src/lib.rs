use crate::helper::{HelperCall, SjsHelpersContainer};
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};

pub mod helper;

deno_core::extension!(
    sjs_helpers,
    esm = ["src/js/helper.ts",],
    state = |state| {
        state.put(SjsHelpersContainer(vec![]));
    }
);

pub fn create_helper_channel(
    max_helper_processing_capacity: usize,
) -> (Sender<HelperCall>, Receiver<HelperCall>) {
    mpsc::channel::<HelperCall>(max_helper_processing_capacity)
}
