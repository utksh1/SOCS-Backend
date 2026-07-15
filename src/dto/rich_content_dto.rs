use serde::{Deserialize, Serialize};
use validator::Validate;

// Project Feature DTOs
#[derive(Debug, Deserialize, Validate)]
pub struct CreateProjectFeatureDto {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    pub description: Option<String>,
    pub display_order: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProjectFeatureDto {
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,
    pub description: Option<String>,
    pub display_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderItemsDto {
    pub items: Vec<ReorderItem>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderItem {
    pub id: String,
    pub display_order: i32,
}

// Project Contributor DTOs
#[derive(Debug, Deserialize, Validate)]
pub struct CreateProjectContributorDto {
    #[validate(length(min = 1))]
    pub team_member_id: String,
    #[validate(length(min = 1, max = 100))]
    pub role: String,
}

// Event Timeline DTOs
#[derive(Debug, Deserialize, Validate)]
pub struct CreateEventTimelineDto {
    #[validate(length(min = 1, max = 10))]
    pub time: String,
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    pub description: Option<String>,
    pub display_order: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateEventTimelineDto {
    pub time: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub display_order: Option<i32>,
}

// Event Prerequisite DTOs
#[derive(Debug, Deserialize, Validate)]
pub struct CreateEventPrerequisiteDto {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    pub description: Option<String>,
    pub display_order: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateEventPrerequisiteDto {
    pub title: Option<String>,
    pub description: Option<String>,
    pub display_order: Option<i32>,
}

// Team Contribution DTOs
#[derive(Debug, Deserialize, Validate)]
pub struct CreateTeamContributionDto {
    #[validate(length(min = 1, max = 50))]
    pub contribution_type: String,
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    pub description: Option<String>,
    #[validate(url)]
    pub url: Option<String>,
    pub contribution_date: String, // YYYY-MM-DD format
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTeamContributionDto {
    pub contribution_type: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub contribution_date: Option<String>,
}
