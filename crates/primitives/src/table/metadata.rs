use deno_core::ModuleId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TableMetadata {
    pub module_id: Option<ModuleId>,
}

impl TableMetadata {
    pub fn set_module_id(&mut self, module_id: ModuleId) {
        self.module_id = Some(module_id);
    }
}
