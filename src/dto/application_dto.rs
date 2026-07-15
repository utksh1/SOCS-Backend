use serde::Deserialize;
use validator::Validate;
use crate::models::application::ApplicationStatus;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateApplicationDto {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    
    #[validate(email)]
    pub email: String,
    
    #[validate(length(min = 1, max = 50))]
    pub experience_level: String,
    
    pub skills: Vec<String>,
    
    pub message: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ReviewApplicationDto {
    pub status: ApplicationStatus,
    
    pub rejection_reason: Option<String>,
}
