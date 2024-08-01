pub mod transpiler;

deno_core::extension!(
    sjs_core,
    esm_entry_point = "ext:sjs_core/src/js/bootstrap.ts",
    esm = [
        "src/js/fieldUtils.ts",
        "src/js/global.ts",
        "src/js/bootstrap.ts",
    ]
);
