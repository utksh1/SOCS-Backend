use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateContactDto {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    
    #[validate(email)]
    pub email: String,
    
    #[validate(length(min = 1, max = 500))]
    pub subject: String,
    
    #[validate(length(min = 1))]
    pub message: String,
}
