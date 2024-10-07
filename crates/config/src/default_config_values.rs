macro_rules! config_constants {
    (
        $(const $name:ident: $type:ty = $value:expr;)*
    ) => {
        $(
            const $name: $type = $value;

            paste::paste! {
                pub fn [<get_ $name:camel>]() -> $type {
                    $name
                }

                pub fn [<str_ $name:camel>]() -> String {
                    $name.to_string()
                }
            }
        )*
    }
}

config_constants! {
    const MAX_TEMPORARY_SHARDS: u64 = 5;
    const MAX_ROWS_PER_TEMP_SHARD: u64 = 1000;
    const MAX_ROWS_PER_SHARD: u64 = 2_500_000;
    const MAX_RECORDS_PER_HASH_INDEX_SHARD: u64 = 10_000_000;
    const DEFAULT_SCHEME_NAME: &'static str = "public";

    const DEFAULT_ROOT_USER: &'static str = "admin";
    const DEFAULT_ROOT_PWD: &'static str = "admin";

    const DEFAULT_GRPC_HOST: &'static str = "[::1]:34244";
    const DEFAULT_CUSTOM_QUERY_TIMEOUT: u64 = 30;

    const DEFAULT_MAX_FILE_DESCRIPTORS: usize = 2500;
}
