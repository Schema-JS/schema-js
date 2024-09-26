fn main() {
    let protos = ["proto/connection/connection.proto"];

    tonic_build::configure()
        .emit_rerun_if_changed(true)
        .compile(&protos, &["proto"])
        .unwrap();
}
