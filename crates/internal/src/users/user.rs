use crate::users::roles::Role;
use schemajs_primitives::column::types::DataTypes;
use schemajs_primitives::column::Column;
use schemajs_primitives::table::Table;

pub struct User {
    pub identifier: String,
    pub hashed_password: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub is_admin: bool,
    pub is_super_admin: bool,
    pub roles: Vec<Role>,
}

pub fn get_internal_user_table() -> Table {
    Table::new("sjs_users")
        .add_column(Column::new("identifier", DataTypes::String).set_required(true))
        .add_column(Column::new("hashed_password", DataTypes::String).set_required(true))
        .add_column(Column::new("created_at", DataTypes::Number))
        .add_column(Column::new("updated_at", DataTypes::Number))
        .add_column(Column::new("is_admin", DataTypes::Boolean).set_default_value("false"))
        .add_column(Column::new("is_super_admin", DataTypes::Boolean).set_default_value("false"))
        .add_column(Column::new("roles", DataTypes::String))
}
