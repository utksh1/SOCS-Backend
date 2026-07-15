use chrono::{DateTime, Utc};
use serde::Deserialize;
use validator::Validate;
use crate::models::blog_post::{PostCategory, PostStatus};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateBlogPostDto {
    #[validate(length(min = 1, max = 255))]
    pub slug: Option<String>,
    
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    
    #[validate(length(min = 1, max = 500))]
    pub excerpt: String,
    
    #[validate(length(min = 1))]
    pub content: String,
    
    pub category: PostCategory,
    
    pub status: Option<PostStatus>,
    
    pub tags: Vec<String>,
    
    #[validate(url)]
    pub featured_image: Option<String>,
    
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateBlogPostDto {
    #[validate(length(min = 1, max = 255))]
    pub slug: Option<String>,
    
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,
    
    #[validate(length(min = 1, max = 500))]
    pub excerpt: Option<String>,
    
    #[validate(length(min = 1))]
    pub content: Option<String>,
    
    pub category: Option<PostCategory>,
    
    pub status: Option<PostStatus>,
    
    pub tags: Option<Vec<String>>,
    
    #[validate(url)]
    pub featured_image: Option<String>,
    
    pub published_at: Option<DateTime<Utc>>,
}
