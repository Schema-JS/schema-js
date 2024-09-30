use crate::users::user::User;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Serialize, Deserialize)]
pub struct VerifyUserArgs {
    pub scheme_name: String,
    pub identifier: String,
    pub password: String,
}

pub struct UserContext {
    user: User,
    authenticated_at: SystemTime,
    last_query_at: Option<SystemTime>,
}

impl UserContext {
    pub fn new(user: User) -> Self {
        Self {
            user,
            authenticated_at: SystemTime::now(),
            last_query_at: None,
        }
    }

    pub fn get_user(&self) -> &User {
        &self.user
    }

    pub fn get_authenticated_at(&self) -> &SystemTime {
        &self.authenticated_at
    }

    pub fn get_last_query_at(&self) -> &Option<SystemTime> {
        &self.last_query_at
    }

    pub fn log_query(&mut self) -> SystemTime {
        let time = SystemTime::now();
        self.last_query_at = Some(time.clone());

        time
    }
}
