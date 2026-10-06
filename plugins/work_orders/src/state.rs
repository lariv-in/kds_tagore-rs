use sea_orm::DatabaseConnection;

/// Runtime state for the KDS Quotations plugin.
#[derive(Clone)]
pub struct WorkOrdersState {
    pub db: DatabaseConnection,
}

impl WorkOrdersState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
