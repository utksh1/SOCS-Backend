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

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateResourceDto {
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,
    
    #[validate(length(min = 1))]
    pub description: Option<String>,
    
    pub category: Option<ResourceCategory>,
    
    #[validate(url)]
    pub url: Option<String>,
    
    pub tags: Option<Vec<String>>,
}
