use crate::users::roles::Role;
use schemajs_primitives::column::types::DataTypes;
use schemajs_primitives::column::Column;
use schemajs_primitives::table::Table;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
pub struct User {
    pub identifier: String,
    pub hashed_password: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub is_admin: bool,
    pub is_super_admin: bool,
    pub roles: Vec<Role>,
    pub scheme: String,
}

pub const INTERNAL_USER_TABLE_NAME: &str = "sjs_users";

pub(crate) static INTERNAL_USER_TABLE: LazyLock<Table> = LazyLock::new(|| {
    let mut tbl = Table::new(INTERNAL_USER_TABLE_NAME)
        .add_column(Column::new("identifier", DataTypes::String).set_required(true))
        .add_column(
            Column::new("scheme", DataTypes::String)
                .set_required(true)
                .set_default_index(true),
        )
        .add_column(
            Column::new("hashed_password", DataTypes::String)
                .set_required(true)
                .set_default_index(false),
        )
        .add_column(Column::new("created_at", DataTypes::Number).set_default_index(false))
        .add_column(Column::new("updated_at", DataTypes::Number).set_default_index(false))
        .add_column(
            Column::new("is_admin", DataTypes::Boolean)
                .set_default_value("false")
                .set_default_index(false),
        )
        .add_column(
            Column::new("is_super_admin", DataTypes::Boolean)
                .set_default_value("false")
                .set_default_index(false),
        )
        .add_column(Column::new("roles", DataTypes::String).set_default_index(false))
        .set_internal(true);

    tbl.init();

    tbl
});

pub fn create_user(
    identifier: String,
    password: String,
    is_admin: bool,
    is_super_admin: bool,
    roles: Vec<Role>,
    scheme: String,
) -> User {
    let hashed_password = bcrypt::hash(password, 12).unwrap();
    let creation_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    User {
        identifier,
        hashed_password,
        created_at: creation_time,
        updated_at: creation_time,
        is_admin,
        is_super_admin,
        roles,
        scheme,
    }
}
