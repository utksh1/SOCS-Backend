use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "visual_category", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VisualCategory {
    Team,
    Infra,
    Event,
    Project,
    Other,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Visual {
    pub id: Uuid,
    pub title: String,
    pub category: VisualCategory,
    pub src: String,
    pub alt_text: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
