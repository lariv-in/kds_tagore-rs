use sea_orm::DatabaseConnection;

/// Runtime state for the Machinery Schedule plugin.
#[derive(Clone)]
pub struct MachineryScheduleState {
    pub db: DatabaseConnection,
}

impl MachineryScheduleState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
