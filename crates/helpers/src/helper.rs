use enum_as_inner::EnumAsInner;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, EnumAsInner, Debug, Clone)]
pub enum HelperType {
    CustomQuery,
    InsertHook,
}

#[derive(Debug)]
pub struct Helper {
    pub identifier: String,
    pub internal_type: HelperType,
    pub func: deno_core::v8::Global<deno_core::v8::Function>,
}

#[derive(Debug, Default)]
pub struct SjsHelpersContainer(pub Vec<Helper>);

pub struct SjsTableHelpers(pub HashMap<String, SjsHelpersContainer>);
