use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeJsWorkspace {
    pub databases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SchemeJsDefault {
    pub scheme_name: String,
    pub username: String,
    pub password: String,
}

impl Default for SchemeJsDefault {
    fn default() -> Self {
        SchemeJsDefault {
            scheme_name: "public".to_string(),
            username: "admin".to_string(),
            password: "admin".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeJsConfig {
    pub workspace: SchemeJsWorkspace,
    pub default: Option<SchemeJsDefault>,
}

impl SchemeJsConfig {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        // Read the TOML file to a string
        let toml = std::fs::read_to_string(path)?;

        // Parse the TOML string to SchemeJsConfig
        let config: SchemeJsConfig = toml::from_str(toml.as_str())?;

        Ok(config)
    }
}
