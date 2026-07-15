use chrono::{DateTime, Utc};
use serde::Deserialize;
use validator::Validate;
use crate::models::event::{EventType, EventStatus};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateEventDto {
    #[validate(length(min = 1, max = 255))]
    pub slug: Option<String>,
    
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    
    #[validate(length(min = 1))]
    pub description: String,
    
    pub date: DateTime<Utc>,
    
    pub event_type: EventType,
    
    pub status: Option<EventStatus>,
    
    #[validate(length(max = 500))]
    pub location: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateEventDto {
    #[validate(length(min = 1, max = 255))]
    pub slug: Option<String>,
    
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,
    
    #[validate(length(min = 1))]
    pub description: Option<String>,
    
    pub date: Option<DateTime<Utc>>,
    
    pub event_type: Option<EventType>,
    
    pub status: Option<EventStatus>,
    
    #[validate(length(max = 500))]
    pub location: Option<String>,
}
