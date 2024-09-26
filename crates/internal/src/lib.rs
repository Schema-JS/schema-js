use schemajs_primitives::table::Table;

pub mod auth;
pub mod manager;
pub mod users;

pub fn get_internal_tables() -> Vec<Table> {
    vec![(&*users::user::INTERNAL_USER_TABLE).clone()]
}
