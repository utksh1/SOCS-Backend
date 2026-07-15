use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct EventRegistration {
    pub id: Uuid,
    pub event_id: Uuid,
    pub name: String,
    pub email: String,
    pub user_id: Option<Uuid>,
    pub attended: bool,
    pub created_at: DateTime<Utc>,
}
