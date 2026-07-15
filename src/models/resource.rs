use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "resource_category", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResourceCategory {
    Tool,
    Roadmap,
    Writeup,
    Guide,
    Other,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Resource {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub category: ResourceCategory,
    pub url: String,
    pub tags: Vec<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
