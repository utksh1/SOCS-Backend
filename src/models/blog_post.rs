use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "post_category", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PostCategory {
    Ctf,
    Writeup,
    Tutorial,
    News,
    Other,
}

#[derive(Debug, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "post_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PostStatus {
    Draft,
    Published,
    Archived,
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, PartialEq, Clone)]
#[sqlx(type_name = "content_status", rename_all = "lowercase")]
pub enum ContentStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct BlogPost {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub content: String,
    pub category: PostCategory,
    pub status: PostStatus,
    pub approval_status: ContentStatus,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
    pub featured_image: Option<String>,
    pub author_id: Option<Uuid>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct BlogCollaborator {
    pub id: Uuid,
    pub blog_post_id: Uuid,
    pub user_id: Uuid,
    pub added_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}
