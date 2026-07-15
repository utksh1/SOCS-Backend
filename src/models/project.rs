use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::Type, PartialEq, Clone)]
#[sqlx(type_name = "content_status", rename_all = "lowercase")]
pub enum ContentStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Project {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub tech_stack: Vec<String>,
    pub github_link: Option<String>,
    pub tags: Vec<String>,
    pub featured: bool,
    pub status: ContentStatus,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProjectCollaborator {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub added_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}
