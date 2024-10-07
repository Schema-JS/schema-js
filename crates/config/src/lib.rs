mod default_config_values;

use crate::default_config_values::{
    get_DefaultCustomQueryTimeout, get_DefaultMaxFileDescriptors, get_MaxRecordsPerHashIndexShard,
    get_MaxRowsPerShard, get_MaxRowsPerTempShard, get_MaxTemporaryShards, str_DefaultGrpcHost,
    str_DefaultRootPwd, str_DefaultRootUser, str_DefaultSchemeName,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SchemeJsConfig {
    pub workspace: SchemeJsWorkspace,
    pub global: GlobalConfig,
    pub db: HashMap<String, DatabaseConfig>,
    pub grpc: GrpcConfig,
    pub process: ProcessConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SchemeJsWorkspace {
    pub databases: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GlobalConfig {
    #[serde(default = "get_MaxTemporaryShards")]
    pub max_temporary_shards: u64,
    #[serde(default = "get_MaxRowsPerTempShard")]
    pub max_rows_per_temp_shard: u64,
    #[serde(default = "get_MaxRowsPerShard")]
    pub max_rows_per_shard: u64,
    #[serde(default = "get_MaxRecordsPerHashIndexShard")]
    pub max_records_per_hash_index_shard: u64,
    #[serde(default)]
    pub default_auth: AuthConfig,
    #[serde(default = "str_DefaultSchemeName")]
    pub default_scheme: String,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            max_temporary_shards: get_MaxTemporaryShards(),
            max_rows_per_temp_shard: get_MaxRowsPerTempShard(),
            max_rows_per_shard: get_MaxRowsPerShard(),
            max_records_per_hash_index_shard: get_MaxRecordsPerHashIndexShard(),
            default_auth: Default::default(),
            default_scheme: str_DefaultSchemeName(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub max_temporary_shards: u64,
    pub max_rows_per_temp_shard: u64,
    pub max_rows_per_shard: u64,
    pub max_records_per_hash_index_shard: u64,
    pub custom_query_timeout: u64,
    pub default_auth: AuthConfig,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            max_temporary_shards: get_MaxTemporaryShards(),
            max_rows_per_temp_shard: get_MaxRowsPerTempShard(),
            max_rows_per_shard: get_MaxRowsPerShard(),
            max_records_per_hash_index_shard: get_MaxRecordsPerHashIndexShard(),
            custom_query_timeout: get_DefaultCustomQueryTimeout(),
            default_auth: Default::default(),
        }
    }
}

impl DatabaseConfig {
    pub fn from_globals(global_config: &GlobalConfig, grpc: &GrpcConfig) -> Self {
        Self {
            max_temporary_shards: global_config.max_temporary_shards,
            max_rows_per_temp_shard: global_config.max_rows_per_temp_shard,
            max_rows_per_shard: global_config.max_rows_per_shard,
            max_records_per_hash_index_shard: global_config.max_records_per_hash_index_shard,
            custom_query_timeout: grpc.custom_query_timeout,
            default_auth: global_config.default_auth.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    #[serde(default = "str_DefaultRootUser")]
    pub username: String,
    #[serde(default = "str_DefaultRootPwd")]
    pub password: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            username: str_DefaultRootUser(),
            password: str_DefaultRootPwd(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct GrpcConfig {
    #[serde(default = "str_DefaultGrpcHost")]
    pub host: String,
    #[serde(default = "get_DefaultCustomQueryTimeout")]
    pub custom_query_timeout: u64,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            host: str_DefaultGrpcHost(),
            custom_query_timeout: get_DefaultCustomQueryTimeout(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProcessConfig {
    #[serde(default = "get_DefaultMaxFileDescriptors")]
    pub max_file_descriptors_in_cache: usize,
}

impl Default for ProcessConfig {
    fn default() -> Self {
        Self {
            max_file_descriptors_in_cache: get_DefaultMaxFileDescriptors(),
        }
    }
}

impl SchemeJsConfig {
    pub fn from_str(toml: &str) -> Result<Self> {
        #[derive(Deserialize, Default)]
        #[serde(default)]
        struct GlobalWrapper {
            #[serde(default)]
            pub workspace: SchemeJsWorkspace,
            #[serde(default)]
            pub global: GlobalConfig,
            #[serde(default)]
            pub grpc: GrpcConfig,
            #[serde(default)]
            pub process: ProcessConfig,
        }

        // Parse the TOML string to SchemeJsConfig
        let global: GlobalWrapper = toml::from_str(toml)?;

        #[derive(Debug, Deserialize, Clone)]
        pub struct OptionalDatabaseConfig {
            pub max_temporary_shards: Option<u64>,
            pub max_rows_per_temp_shard: Option<u64>,
            pub max_rows_per_shard: Option<u64>,
            pub max_records_per_hash_index_shard: Option<u64>,
            pub custom_query_timeout: Option<u64>,
            pub default_auth: Option<AuthConfig>,
        }

        #[derive(Deserialize, Default)]
        #[serde(default)]
        pub struct DbWrapper {
            #[serde(default)]
            pub db: HashMap<String, OptionalDatabaseConfig>,
        }

        let db: DbWrapper = toml::from_str(toml)?;

        let mut actual_db: HashMap<String, DatabaseConfig> = HashMap::new();
        {
            for (key, val) in db.db {
                actual_db.insert(
                    key,
                    DatabaseConfig {
                        max_temporary_shards: val
                            .max_temporary_shards
                            .unwrap_or_else(|| global.global.max_temporary_shards),
                        max_rows_per_temp_shard: val
                            .max_rows_per_temp_shard
                            .unwrap_or_else(|| global.global.max_rows_per_temp_shard),
                        max_rows_per_shard: val
                            .max_rows_per_shard
                            .unwrap_or_else(|| global.global.max_rows_per_shard),
                        max_records_per_hash_index_shard: val
                            .max_records_per_hash_index_shard
                            .unwrap_or_else(|| global.global.max_records_per_hash_index_shard),
                        custom_query_timeout: val
                            .custom_query_timeout
                            .unwrap_or_else(|| global.grpc.custom_query_timeout),
                        default_auth: val
                            .default_auth
                            .unwrap_or_else(|| global.global.default_auth.clone()),
                    },
                );
            }
        }

        Ok(SchemeJsConfig {
            workspace: global.workspace,
            global: global.global,
            db: actual_db,
            grpc: global.grpc,
            process: global.process,
        })
    }

    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        // Read the TOML file to a string
        let toml = std::fs::read_to_string(path)?;

        Self::from_str(toml.as_str())
    }

    pub fn global_config(&self) -> &GlobalConfig {
        &self.global
    }

    pub fn db_config(&self, db_name: &str) -> DatabaseConfig {
        self.db.get(db_name).map(|r| r.clone()).unwrap_or_else(|| {
            DatabaseConfig::from_globals(self.global_config(), self.grpc_config())
        })
    }

    pub fn grpc_config(&self) -> &GrpcConfig {
        &self.grpc
    }
}

#[cfg(test)]
mod tests {
    use crate::default_config_values::{get_DefaultRootPwd, get_MaxTemporaryShards};
    use crate::SchemeJsConfig;

    #[test]
    fn test_toml_config() {
        let config: SchemeJsConfig = SchemeJsConfig::from_str(
            r#"
  [db.public]
  custom_query_timeout = 1
  [db.public.default_auth]
  username = "lion"
"#,
        )
        .unwrap();

        let db = config.db.get("public").unwrap();

        let public_db = db.custom_query_timeout;
        assert_eq!(public_db, 1u64);
        assert_eq!(db.max_temporary_shards, get_MaxTemporaryShards());

        assert_eq!(db.default_auth.username, "lion");
        assert_eq!(db.default_auth.password, get_DefaultRootPwd());
    }
}
