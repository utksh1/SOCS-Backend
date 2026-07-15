use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "event_type", rename_all = "lowercase")]
pub enum EventType {
    Workshop,
    Ctf,
    Talk,
    Hackathon,
    Other,
}

#[derive(Debug, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "event_status", rename_all = "lowercase")]
pub enum EventStatus {
    Upcoming,
    Ongoing,
    Past,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Event {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub date: DateTime<Utc>,
    #[sqlx(rename = "type")]
    pub event_type: EventType,
    pub status: EventStatus,
    pub location: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
