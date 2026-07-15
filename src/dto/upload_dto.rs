use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct DeleteImageDto {
    #[validate(url)]
    pub url: String,
}
