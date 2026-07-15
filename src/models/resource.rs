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

#[derive(Debug, Serialize, Deserialize, sqlx::Type, PartialEq, Clone)]
#[sqlx(type_name = "content_status", rename_all = "lowercase")]
pub enum ContentStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Resource {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub category: ResourceCategory,
    pub url: String,
    pub tags: Vec<String>,
    pub status: ContentStatus,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ResourceCollaborator {
    pub id: Uuid,
    pub resource_id: Uuid,
    pub user_id: Uuid,
    pub added_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}
