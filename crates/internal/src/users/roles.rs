use serde::{Deserialize, Serialize};

/// Enum representing different levels of access within the system.
#[derive(Serialize, Deserialize)]
pub enum Role {
    /// Permissions for specific tables, encapsulated in a container for clarity.
    TablePermissions(Vec<PermissionTableContainer>),

    /// Permissions for all tables. They are lower in priority against `TablePermissions`
    GlobalTablePermissions(Vec<TableAction>),

    /// Can create users in the system.
    CanCreateUsers,

    /// Can create new tables in the system.
    CanCreateTables,

    /// Can create new databases in the system.
    CanCreateDatabase,
}

/// Struct representing permissions for a specific table.
#[derive(Serialize, Deserialize)]
pub struct PermissionTableContainer {
    pub table_name: String,        // Name of the table
    pub actions: Vec<TableAction>, // Actions allowed on this table
}

/// Enum representing possible actions a user can take on a table.
#[derive(Serialize, Deserialize)]
pub enum TableAction {
    View,   // Read/query data from the table
    Modify, // Insert, update, or delete data
    Delete, // Delete data from the table
    Update, // Update data in the table
}
