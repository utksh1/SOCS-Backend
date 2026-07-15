use aws_sdk_s3::{Client, Config};
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::primitives::ByteStream;
use std::env;
use uuid::Uuid;

use crate::error::ApiError;

pub struct R2Client {
    client: Client,
    bucket_name: String,
    public_url: String,
}

impl R2Client {
    pub fn new() -> Result<Self, ApiError> {
        let missing_config = || {
            ApiError::ServiceUnavailable("Image uploads are not configured".to_string())
        };
        let account_id = env::var("R2_ACCOUNT_ID").map_err(|_| missing_config())?;
        let access_key = env::var("R2_ACCESS_KEY_ID").map_err(|_| missing_config())?;
        let secret_key = env::var("R2_SECRET_ACCESS_KEY").map_err(|_| missing_config())?;
        let bucket_name = env::var("R2_BUCKET_NAME").unwrap_or_else(|_| "socs-images".to_string());
        let public_url = env::var("R2_PUBLIC_URL").map_err(|_| missing_config())?;

        let endpoint = format!("https://{}.r2.cloudflarestorage.com", account_id);
        
        let credentials = Credentials::new(
            access_key,
            secret_key,
            None,
            None,
            "r2-credentials",
        );

        let config = Config::builder()
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .credentials_provider(credentials)
            .endpoint_url(endpoint)
            .region(aws_sdk_s3::config::Region::new("auto"))
            .build();

        let client = Client::from_conf(config);

        Ok(Self {
            client,
            bucket_name,
            public_url: public_url.trim_end_matches('/').to_string(),
        })
    }

    /// Upload a file to R2 and return the public URL
    pub async fn upload_image(
        &self,
        file_data: Vec<u8>,
        content_type: &str,
        folder: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Generate unique filename
        let extension = match content_type {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => "jpg",
        };
        
        let filename = format!("{}/{}.{}", folder, Uuid::new_v4(), extension);

        // Upload to R2
        self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(&filename)
            .body(ByteStream::from(file_data))
            .content_type(content_type)
            .send()
            .await?;

        // Return public URL
        Ok(format!("{}/{}", self.public_url, filename))
    }

    /// Delete a file from R2
    pub async fn delete_image(&self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        let key = self.object_key(url).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "URL does not belong to the configured image bucket",
            )
        })?;

        self.client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await?;

        Ok(())
    }

    /// Check whether a returned public URL belongs to a user-scoped folder.
    /// The API has no generic upload table, so the unguessable user-id prefix
    /// is the authorization boundary for direct image deletion.
    pub fn url_is_in_folder(&self, url: &str, folder: &str) -> bool {
        let folder = folder.trim_matches('/');
        self.object_key(url)
            .is_some_and(|key| key.starts_with(&format!("{folder}/")))
    }

    fn object_key<'a>(&self, url: &'a str) -> Option<&'a str> {
        let prefix = format!("{}/", self.public_url);
        let key = url.strip_prefix(&prefix)?;
        if key.is_empty()
            || key.contains(['?', '#'])
            || key.split('/').any(|segment| matches!(segment, "." | ".."))
        {
            return None;
        }
        Some(key)
    }
}
