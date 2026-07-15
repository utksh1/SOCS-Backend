use serde::Deserialize;
use validator::Validate;
use crate::models::team::MemberTier;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateTeamMemberDto {
    #[validate(length(min = 1, max = 255))]
    pub slug: Option<String>,
    
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    
    #[validate(length(min = 1, max = 255))]
    pub role: String,
    
    #[validate(length(max = 2000))]
    pub bio: Option<String>,
    
    pub skills: Vec<String>,
    
    pub tier: Option<MemberTier>,
    
    #[validate(url)]
    pub github: Option<String>,
    
    #[validate(url)]
    pub linkedin: Option<String>,
    
    #[validate(url)]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTeamMemberDto {
    #[validate(length(min = 1, max = 255))]
    pub slug: Option<String>,
    
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    
    #[validate(length(min = 1, max = 255))]
    pub role: Option<String>,
    
    pub skills: Option<Vec<String>>,
    
    pub tier: Option<MemberTier>,
    
    #[validate(url)]
    pub github: Option<String>,
    
    #[validate(url)]
    pub linkedin: Option<String>,
    
    #[validate(url)]
    pub avatar_url: Option<String>,
}
