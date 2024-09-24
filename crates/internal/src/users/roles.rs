/// Enum representing different levels of access within the system.
pub enum Role {
    /// Full access to query all data in the system.
    AllQuery,

    /// Permissions for specific tables, encapsulated in a container for clarity.
    TablePermissions(Vec<PermissionTableContainer>),

    /// Can create users in the system.
    CanCreateUsers,

    /// Can create new tables in the system.
    CanCreateTables,

    /// Can create new databases in the system.
    CanCreateDatabase,
}

/// Struct representing permissions for a specific table.
pub struct PermissionTableContainer {
    pub table_name: String,        // Name of the table
    pub actions: Vec<TableAction>, // Actions allowed on this table
}

/// Enum representing possible actions a user can take on a table.
pub enum TableAction {
    View,   // Read/query data from the table
    Modify, // Insert, update, or delete data
    Delete, // Delete data from the table
    Update, // Update data in the table
}
