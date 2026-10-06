use sea_orm::DatabaseConnection;

/// Runtime state for the Delivery plugin.
#[derive(Clone)]
pub struct DeliveryState {
    pub db: DatabaseConnection,
}

impl DeliveryState {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
