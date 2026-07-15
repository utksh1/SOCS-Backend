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
    
    // Restore the user
    let result = sqlx::query(
        "UPDATE users SET deleted_at = NULL WHERE id = $1"
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("User not found".into()));
    }
    
    // Restore cascaded team_contributions
    sqlx::query(
        "UPDATE team_contributions SET deleted_at = NULL WHERE team_member_id = $1"
    )
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
    
    // Permanently delete the user
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound("User not found".into()));
    }
    
    tx.commit().await?;
    
    Ok(())
}
