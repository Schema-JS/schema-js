use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeJsWorkspace {
    pub databases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeJsConfig {
    pub workspace: SchemeJsWorkspace,
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
