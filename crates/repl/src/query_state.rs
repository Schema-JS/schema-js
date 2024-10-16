pub enum ReplQueryState {
    Global,
    Database(String),      // Holds the current database name
    Table(String, String), // Holds both the current database and table names
}
