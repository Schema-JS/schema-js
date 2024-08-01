use crate::database::Database;

pub struct Collection {
    pub dbs: Vec<Database>,
}

impl Collection {
    pub fn new() -> Self {
        Self { dbs: vec![] }
    }

    pub fn add_database(&mut self, database: Database) {
        self.dbs.push(database);
    }
}
