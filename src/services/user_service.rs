use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiError;

pub async fn soft_delete_user(pool: &PgPool, id: Uuid) -> Result<(), ApiError> {
    let mut tx = pool.begin().await?;
    
    // Soft delete the user
    let result = sqlx::query(
        "UPDATE users SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("User not found or already deleted".into()));
    }

    // A soft-deleted account must not retain usable verification or password
    // reset links. New links can be issued after an administrator restores it.
    sqlx::query("DELETE FROM verification_tokens WHERE user_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    
    // Cascade to team_contributions (intrinsic to user)
    sqlx::query(
        "UPDATE team_contributions SET deleted_at = NOW() 
         WHERE team_member_id = $1 AND deleted_at IS NULL"
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    
    // Other associations like project_collaborators, project_contributors, 
    // event_registrations are NOT cascaded (historical records)
    
    tx.commit().await?;
    
    Ok(())
}

pub async fn restore_user(pool: &PgPool, id: Uuid) -> Result<(), ApiError> {
    let mut tx = pool.begin().await?;

    // Lock the deleted parent first. PostgreSQL's NOW() is stable within the
    // soft-delete transaction, so this timestamp identifies only records
    // that were cascaded with the parent—not child records deleted earlier on
    // their own.
    let deleted_at = sqlx::query_scalar::<_, chrono::DateTime<chrono::Utc>>(
        "SELECT deleted_at FROM users WHERE id = $1 AND deleted_at IS NOT NULL FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::NotFound("User not found or not deleted".into()))?;

    // Restore only records that were cascaded when this account was deleted.
    sqlx::query(
        "UPDATE team_contributions SET deleted_at = NULL WHERE team_member_id = $1 AND deleted_at = $2"
    )
    .bind(id)
    .bind(deleted_at)
    .execute(&mut *tx)
    .await?;

    sqlx::query("UPDATE users SET deleted_at = NULL WHERE id = $1 AND deleted_at IS NOT NULL")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    
    tx.commit().await?;
    
    Ok(())
}

pub async fn permanent_delete_user(pool: &PgPool, id: Uuid) -> Result<(), ApiError> {
    let mut tx = pool.begin().await?;
    
    // Permanently delete cascaded records first
    sqlx::query("DELETE FROM team_contributions WHERE team_member_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    // These are associations, not user-owned content. A permanent deletion is
    // explicitly destructive, so remove them before the user FK is removed.
    sqlx::query("DELETE FROM project_contributors WHERE team_member_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM project_collaborators WHERE user_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM resource_collaborators WHERE user_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM blog_collaborators WHERE user_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    // announcements.author_id is a required legacy foreign key. Permanent
    // deletion is explicitly destructive, so remove authored announcements
    // before deleting the account rather than leaving a FK violation.
    sqlx::query("DELETE FROM announcements WHERE author_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    
    // Permanently delete the user
    let result = sqlx::query("DELETE FROM users WHERE id = $1 AND deleted_at IS NOT NULL")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    
    if result.rows_affected() == 0 {
        return Err(ApiError::Conflict(
            "User must be soft-deleted before it can be permanently removed".into(),
        ));
    }
    
    tx.commit().await?;
    
    Ok(())
}
