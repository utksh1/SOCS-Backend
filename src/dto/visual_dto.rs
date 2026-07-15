use serde::Deserialize;
use validator::Validate;
use crate::models::visual::VisualCategory;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateVisualDto {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    
    pub category: VisualCategory,
    
    #[validate(url)]
    pub src: String,
    
    #[validate(length(max = 500))]
    pub alt_text: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateVisualDto {
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,
    
    pub category: Option<VisualCategory>,
    
    #[validate(url)]
    pub src: Option<String>,
    
    #[validate(length(max = 500))]
    pub alt_text: Option<String>,
}
