use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

pub async fn soft_delete_event(pool: &PgPool, id: Uuid) -> Result<(), ApiError> {
    let mut tx = pool.begin().await?;
    
    // Soft delete the event
    let result = sqlx::query(
        "UPDATE events SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("Event not found or already deleted".into()));
    }
    
    // Cascade to event_timeline_items
    sqlx::query(
        "UPDATE event_timeline_items SET deleted_at = NOW() 
         WHERE event_id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    
    // Cascade to event_prerequisites
    sqlx::query(
        "UPDATE event_prerequisites SET deleted_at = NOW() 
         WHERE event_id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    
    // event_registrations are NOT cascaded (historical attendance data)
    
    tx.commit().await?;
    
    Ok(())
}

pub async fn restore_event(pool: &PgPool, id: Uuid) -> Result<(), ApiError> {
    let mut tx = pool.begin().await?;

    let deleted_at = sqlx::query_scalar::<_, chrono::DateTime<chrono::Utc>>(
        "SELECT deleted_at FROM events WHERE id = $1 AND deleted_at IS NOT NULL FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::NotFound("Event not found or not deleted".into()))?;

    // Only restore records that were deleted as part of this parent cascade.
    sqlx::query(
        "UPDATE event_timeline_items SET deleted_at = NULL WHERE event_id = $1 AND deleted_at = $2"
    )
    .bind(id)
    .bind(deleted_at)
    .execute(&mut *tx)
    .await?;
    
    sqlx::query(
        "UPDATE event_prerequisites SET deleted_at = NULL WHERE event_id = $1 AND deleted_at = $2"
    )
    .bind(id)
    .bind(deleted_at)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE events SET deleted_at = NULL WHERE id = $1 AND deleted_at IS NOT NULL")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    
    tx.commit().await?;
    
    Ok(())
}

pub async fn permanent_delete_event(pool: &PgPool, id: Uuid) -> Result<(), ApiError> {
    let mut tx = pool.begin().await?;
    
    // Permanently delete cascaded records first
    sqlx::query("DELETE FROM event_timeline_items WHERE event_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    
    sqlx::query("DELETE FROM event_prerequisites WHERE event_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    
    // Permanently delete the event
    let result = sqlx::query("DELETE FROM events WHERE id = $1 AND deleted_at IS NOT NULL")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    
    if result.rows_affected() == 0 {
        return Err(ApiError::Conflict(
            "Event must be soft-deleted before it can be permanently removed".into(),
        ));
    }
    
    tx.commit().await?;
    
    Ok(())
}
