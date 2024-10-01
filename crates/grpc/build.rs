fn main() {
    let protos = [
        "proto/connection/connection.proto",
        "proto/shared/data_value.proto",
        "proto/shared/row.proto",
        "proto/query/query.proto",
    ];

    tonic_build::configure()
        .type_attribute(".", "#[derive(serde::Deserialize, serde::Serialize)]")
        .emit_rerun_if_changed(true)
        .compile(&protos, &["./proto"])
        .unwrap();
}
