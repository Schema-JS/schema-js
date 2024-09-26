use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct VerifyUserArgs {
    pub scheme_name: String,
    pub identifier: String,
    pub password: String,
}
