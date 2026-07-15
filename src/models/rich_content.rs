use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Project Feature
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProjectFeature {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Project Contributor
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProjectContributor {
    pub id: Uuid,
    pub project_id: Uuid,
    pub team_member_id: Uuid,
    pub role: String,
    pub joined_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// Event Timeline Item
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct EventTimelineItem {
    pub id: Uuid,
    pub event_id: Uuid,
    pub time: String,
    pub title: String,
    pub description: Option<String>,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Event Prerequisite
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct EventPrerequisite {
    pub id: Uuid,
    pub event_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Team Contribution
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamContribution {
    pub id: Uuid,
    pub team_member_id: Uuid,
    pub contribution_type: String,
    pub title: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub contribution_date: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Response types with joined data
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProjectContributorWithMember {
    pub id: Uuid,
    pub project_id: Uuid,
    pub team_member_id: Uuid,
    pub role: String,
    pub joined_at: DateTime<Utc>,
    pub member_name: String,
    pub member_avatar: Option<String>,
    pub member_slug: String,
}
