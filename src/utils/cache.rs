//! Best-effort Redis caching for public read models.
//!
//! Cache failures intentionally fall back to the database. Mutations bump a
//! per-resource version so old page keys naturally become unreachable while
//! their short TTLs expire.

use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};

use crate::AppState;

const PUBLIC_CACHE_TTL_SECONDS: u64 = 30;
const VERSION_TTL_SECONDS: i64 = 24 * 60 * 60;

pub async fn public_list_key(
    state: &AppState,
    resource: &str,
    page: i64,
    limit: i64,
) -> Option<String> {
    let mut redis = state.redis.clone()?;
    let version_key = format!("cache:public:{resource}:version");
    let version: Option<u64> = redis.get(&version_key).await.ok()?;
    Some(format!(
        "cache:public:{resource}:v{}:page:{page}:limit:{limit}",
        version.unwrap_or(0)
    ))
}

pub async fn get_json<T: DeserializeOwned>(state: &AppState, key: &str) -> Option<T> {
    let mut redis = state.redis.clone()?;
    let payload: Option<String> = redis.get(key).await.ok()?;
    payload.and_then(|payload| serde_json::from_str(&payload).ok())
}

pub async fn set_json<T: Serialize>(state: &AppState, key: &str, value: &T) {
    let Some(mut redis) = state.redis.clone() else {
        return;
    };
    let Ok(payload) = serde_json::to_string(value) else {
        tracing::warn!(cache_key = key, "Failed to serialize public cache response");
        return;
    };

    let result: redis::RedisResult<()> = redis.set_ex(key, payload, PUBLIC_CACHE_TTL_SECONDS).await;
    if let Err(error) = result {
        tracing::debug!(%error, cache_key = key, "Failed to populate public cache");
    }
}

pub async fn invalidate_public_resource(state: &AppState, resource: &str) {
    let Some(mut redis) = state.redis.clone() else {
        return;
    };
    let version_key = format!("cache:public:{resource}:version");
    let result: redis::RedisResult<i64> = redis.incr(&version_key, 1).await;
    match result {
        Ok(_) => {
            let _: redis::RedisResult<bool> = redis.expire(&version_key, VERSION_TTL_SECONDS).await;
        }
        Err(error) => {
            tracing::debug!(%error, resource, "Failed to invalidate public cache");
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn public_keys_are_resource_and_page_scoped() {
        let event_page = "cache:public:events:v2:page:1:limit:10";
        let project_page = "cache:public:projects:v2:page:1:limit:10";
        assert_ne!(event_page, project_page);
        assert_ne!(event_page, "cache:public:events:v2:page:2:limit:10");
    }
}
