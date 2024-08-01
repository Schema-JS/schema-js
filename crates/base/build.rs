use std::env;
use std::path::PathBuf;

mod schema_js_snapshot {
    use deno_core::snapshot::{create_snapshot, CreateSnapshotOptions};
    use deno_core::Extension;
    use schemajs_core::transpiler::maybe_transpile_source;
    use schemajs_engine::engine::SchemeJsEngine;
    use std::cell::RefCell;
    use std::io::Write;
    use std::path::PathBuf;
    use std::rc::Rc;
    use std::sync::Arc;

    pub fn create_runtime_snapshot(snapshot_path: PathBuf) {
        let extensions: Vec<Extension> = vec![
            schemajs_core::sjs_core::init_ops_and_esm(),
            schemajs_primitives::sjs_primitives::init_ops_and_esm(),
            schemajs_engine::sjs_engine::init_ops_and_esm(Arc::new(RefCell::new(
                SchemeJsEngine::new(),
            ))),
        ];
        let snapshot = create_snapshot(
            CreateSnapshotOptions {
                cargo_manifest_dir: env!("CARGO_MANIFEST_DIR"),
                startup_snapshot: None,
                skip_op_registration: false,
                extensions,
                extension_transpiler: Some(Rc::new(|specifier, source| {
                    maybe_transpile_source(specifier, source)
                })),
                with_runtime_cb: None,
            },
            None,
        );

        let output = snapshot.unwrap();

        let mut snapshot = std::fs::File::create(snapshot_path).unwrap();
        snapshot.write_all(&output.output).unwrap();

        for path in output.files_loaded_during_snapshot {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}
fn main() {
    println!("cargo:rustc-env=TARGET={}", env::var("TARGET").unwrap());
    println!("cargo:rustc-env=PROFILE={}", env::var("PROFILE").unwrap());

    let o = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // Main snapshot
    let runtime_snapshot_path = o.join("RUNTIME_SNAPSHOT.bin");

    schema_js_snapshot::create_runtime_snapshot(runtime_snapshot_path.clone());
}
