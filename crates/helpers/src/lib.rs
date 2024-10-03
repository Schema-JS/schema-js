use crate::helper::SjsHelpersContainer;

pub mod helper;

deno_core::extension!(
    sjs_helpers,
    esm = ["src/js/helper.ts",],
    state = |state| {
        state.put(SjsHelpersContainer(vec![]));
    }
);
