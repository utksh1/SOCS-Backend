use sqlx::PgPool;
use uuid::Uuid;

use crate::models::resource::Resource;



pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Resource>, sqlx::Error> {
    sqlx::query_as::<_, Resource>(
        r#"
        SELECT id, title, description, category, url, tags, created_by, created_at, updated_at
        FROM resources
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM resources WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    
    Ok(result.rows_affected() > 0)
}
