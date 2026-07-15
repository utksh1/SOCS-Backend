use serde::Deserialize;

fn normalize_cors_origins(value: &str) -> Result<String, String> {
    let mut origins = Vec::new();

    for raw_origin in value.split(',').map(str::trim).filter(|origin| !origin.is_empty()) {
        if raw_origin == "*" {
            return Err("CORS_ORIGIN cannot use '*' while credentialed requests are enabled".to_string());
        }

        let uri = raw_origin
            .parse::<http::Uri>()
            .map_err(|_| format!("CORS_ORIGIN contains an invalid origin: {raw_origin}"))?;
        let scheme = uri
            .scheme_str()
            .filter(|scheme| matches!(*scheme, "http" | "https"))
            .ok_or_else(|| format!("CORS_ORIGIN must use http or https: {raw_origin}"))?;
        let authority = uri
            .authority()
            .filter(|authority| !authority.as_str().contains('@'))
            .ok_or_else(|| format!("CORS_ORIGIN must include a host: {raw_origin}"))?;
        if uri
            .path_and_query()
            .is_some_and(|path_and_query| path_and_query.as_str() != "/")
        {
            return Err(format!(
                "CORS_ORIGIN entries must be origins, not paths or queries: {raw_origin}"
            ));
        }

        let origin = format!("{scheme}://{authority}");
        if !origins.contains(&origin) {
            origins.push(origin);
        }
    }

    if origins.is_empty() {
        return Err("CORS_ORIGIN must contain at least one http(s) origin".to_string());
    }

    Ok(origins.join(","))
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub jwt_secret: String,
    pub jwt_expires_in: i64,
    pub cors_origin: String,
    pub redis_url: String,
    pub rate_limit_enabled: bool,
    pub frontend_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        dotenvy::dotenv().ok();
        let cors_origin = std::env::var("CORS_ORIGIN")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());
        
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .map_err(|_| "DATABASE_URL must be set".to_string())?,
            host: std::env::var("HOST")
                .unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "5001".to_string())
                .parse()
                .map_err(|_| "PORT must be a valid u16".to_string())?,
            jwt_secret: std::env::var("JWT_SECRET")
                .map_err(|_| "JWT_SECRET must be set".to_string())?,
            jwt_expires_in: std::env::var("JWT_EXPIRES_IN")
                .unwrap_or_else(|_| "604800".to_string())
                .parse()
                .map_err(|_| "JWT_EXPIRES_IN must be a valid i64".to_string())?,
            cors_origin: normalize_cors_origins(&cors_origin)?,
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            rate_limit_enabled: std::env::var("RATE_LIMIT_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .map_err(|_| "RATE_LIMIT_ENABLED must be true or false".to_string())?,
            frontend_url: std::env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
        })
    }
}
