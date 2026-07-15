use serde::Deserialize;
use validator::Validate;
use crate::models::resource::ResourceCategory;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateResourceDto {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    
    #[validate(length(min = 1))]
    pub description: String,
    
    pub category: ResourceCategory,
    
    #[validate(url)]
    pub url: String,
    
    pub tags: Vec<String>,
}

