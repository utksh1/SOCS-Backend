use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateProjectDto {
    #[validate(length(min = 1, max = 255))]
    pub slug: Option<String>,
    
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    
    #[validate(length(min = 1))]
    pub description: String,
    
    pub tech_stack: Vec<String>,
    
    #[validate(url)]
    pub github_link: Option<String>,
    
    pub tags: Vec<String>,
    
    pub featured: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProjectDto {
    #[validate(length(min = 1, max = 255))]
    pub slug: Option<String>,
    
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,
    
    #[validate(length(min = 1))]
    pub description: Option<String>,
    
    pub tech_stack: Option<Vec<String>>,
    
    #[validate(url)]
    pub github_link: Option<String>,
    
    pub tags: Option<Vec<String>>,
    
    pub featured: Option<bool>,
}
