use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

pub async fn soft_delete_project(pool: &PgPool, id: Uuid) -> Result<(), ApiError> {
    let mut tx = pool.begin().await?;
    
    // Soft delete the project
    let result = sqlx::query(
        "UPDATE projects SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Project not found or already deleted".into()));
    }
    
    // Cascade to project_features (intrinsic to project)
    sqlx::query(
        "UPDATE project_features SET deleted_at = NOW() 
         WHERE project_id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    
    // project_contributors and project_collaborators are NOT cascaded (historical records)
    
    tx.commit().await?;
    
    Ok(())
}

pub async fn restore_project(pool: &PgPool, id: Uuid) -> Result<(), ApiError> {
    let mut tx = pool.begin().await?;

    let deleted_at = sqlx::query_scalar::<_, chrono::DateTime<chrono::Utc>>(
        "SELECT deleted_at FROM projects WHERE id = $1 AND deleted_at IS NOT NULL FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::NotFound("Project not found or not deleted".into()))?;

    // Restore only features that were deleted by the parent cascade. This
    // preserves features an author intentionally deleted before the project.
    sqlx::query(
        "UPDATE project_features SET deleted_at = NULL WHERE project_id = $1 AND deleted_at = $2"
    )
    .bind(id)
    .bind(deleted_at)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE projects SET deleted_at = NULL WHERE id = $1 AND deleted_at IS NOT NULL")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    
    tx.commit().await?;
    
    Ok(())
}

pub async fn permanent_delete_project(pool: &PgPool, id: Uuid) -> Result<(), ApiError> {
    let mut tx = pool.begin().await?;
    
    // Permanently delete cascaded records first
    sqlx::query("DELETE FROM project_features WHERE project_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    
    // Permanently delete the project
    let result = sqlx::query("DELETE FROM projects WHERE id = $1 AND deleted_at IS NOT NULL")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    
    if result.rows_affected() == 0 {
        return Err(ApiError::Conflict(
            "Project must be soft-deleted before it can be permanently removed".into(),
        ));
    }
    
    tx.commit().await?;
    
    Ok(())
}
