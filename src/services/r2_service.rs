use aws_sdk_s3::{Client, Config};
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::primitives::ByteStream;
use uuid::Uuid;
use std::env;

pub struct R2Client {
    client: Client,
    bucket_name: String,
    public_url: String,
}

impl R2Client {
    pub fn new() -> Self {
        let account_id = env::var("R2_ACCOUNT_ID").expect("R2_ACCOUNT_ID must be set");
        let access_key = env::var("R2_ACCESS_KEY_ID").expect("R2_ACCESS_KEY_ID must be set");
        let secret_key = env::var("R2_SECRET_ACCESS_KEY").expect("R2_SECRET_ACCESS_KEY must be set");
        let bucket_name = env::var("R2_BUCKET_NAME").unwrap_or_else(|_| "socs-images".to_string());
        let public_url = env::var("R2_PUBLIC_URL").expect("R2_PUBLIC_URL must be set");

        let endpoint = format!("https://{}.r2.cloudflarestorage.com", account_id);
        
        let credentials = Credentials::new(
            access_key,
            secret_key,
            None,
            None,
            "r2-credentials",
        );

        let config = Config::builder()
            .credentials_provider(credentials)
            .endpoint_url(endpoint)
            .region(aws_sdk_s3::config::Region::new("auto"))
            .build();

        let client = Client::from_conf(config);

        Self {
            client,
            bucket_name,
            public_url,
        }
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
        // Extract key from URL
        let key = url.replace(&format!("{}/", self.public_url), "");

        self.client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(&key)
            .send()
            .await?;

        Ok(())
    }
}
