use chrono::{Duration, Utc};
use serde::Serialize;
use sqlx::PgPool;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};
use uuid::Uuid;

use crate::repositories::{
    announcement_repository, application_repository, blog_post_repository, contact_repository,
    event_registration_repository, notification_repository,
    resource_repository, visual_repository,
};
use crate::services::{event_service, project_service};

#[derive(Debug, Serialize)]
pub struct CleanupStats {
    pub projects_purged: usize,
    pub events_purged: usize,
    pub announcements_purged: usize,
    pub applications_purged: usize,
    pub blog_posts_purged: usize,
    pub contacts_purged: usize,
    pub event_registrations_purged: usize,
    pub notifications_purged: usize,
    pub resources_purged: usize,
    pub visuals_purged: usize,
    pub total_purged: usize,
}

/// Start the cleanup scheduler that runs daily at 2 AM
pub async fn start_cleanup_scheduler(pool: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let scheduler = JobScheduler::new().await?;

    let job = Job::new_async("0 0 2 * * *", move |_uuid, _l| {
        let pool = pool.clone();
        Box::pin(async move {
            info!("Starting scheduled cleanup of expired soft-deleted records");
            match cleanup_expired_soft_deletes(&pool).await {
                Ok(stats) => {
                    info!(
                        "Cleanup completed successfully: {:?}",
                        stats
                    );
                }
                Err(e) => {
                    error!("Cleanup failed: {}", e);
                }
            }
        })
    })?;

    scheduler.add(job).await?;
    scheduler.start().await?;

    info!("Cleanup scheduler started (runs daily at 2 AM)");
    Ok(())
}

/// Clean up soft-deleted records older than 30 days
pub async fn cleanup_expired_soft_deletes(pool: &PgPool) -> Result<CleanupStats, sqlx::Error> {
    let cutoff_date = Utc::now() - Duration::days(30);

    let mut stats = CleanupStats {
        projects_purged: 0,
        events_purged: 0,
        announcements_purged: 0,
        applications_purged: 0,
        blog_posts_purged: 0,
        contacts_purged: 0,
        event_registrations_purged: 0,
        notifications_purged: 0,
        resources_purged: 0,
        visuals_purged: 0,
        total_purged: 0,
    };

    // Clean up projects (cascade delete)
    let project_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM projects WHERE deleted_at IS NOT NULL AND deleted_at < $1",
    )
    .bind(cutoff_date)
    .fetch_all(pool)
    .await?;

    for id in project_ids {
        if let Err(e) = project_service::permanent_delete_project(pool, id).await {
            error!("Failed to permanently delete project {}: {:?}", id, e);
        } else {
            stats.projects_purged += 1;
        }
    }

    // Clean up events (cascade delete)
    let event_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM events WHERE deleted_at IS NOT NULL AND deleted_at < $1",
    )
    .bind(cutoff_date)
    .fetch_all(pool)
    .await?;

    for id in event_ids {
        if let Err(e) = event_service::permanent_delete_event(pool, id).await {
            error!("Failed to permanently delete event {}: {:?}", id, e);
        } else {
            stats.events_purged += 1;
        }
    }

    // Clean up announcements
    let announcement_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM announcements WHERE deleted_at IS NOT NULL AND deleted_at < $1",
    )
    .bind(cutoff_date)
    .fetch_all(pool)
    .await?;

    for id in announcement_ids {
        match announcement_repository::permanent_delete(pool, id).await {
            Ok(true) => stats.announcements_purged += 1,
            Ok(false) => {}
            Err(e) => error!("Failed to permanently delete announcement {}: {}", id, e),
        }
    }

    // Clean up applications
    let application_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM applications WHERE deleted_at IS NOT NULL AND deleted_at < $1",
    )
    .bind(cutoff_date)
    .fetch_all(pool)
    .await?;

    for id in application_ids {
        match application_repository::permanent_delete(pool, id).await {
            Ok(true) => stats.applications_purged += 1,
            Ok(false) => {}
            Err(e) => error!("Failed to permanently delete application {}: {}", id, e),
        }
    }

    // Clean up blog posts
    let blog_post_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM blog_posts WHERE deleted_at IS NOT NULL AND deleted_at < $1",
    )
    .bind(cutoff_date)
    .fetch_all(pool)
    .await?;

    for id in blog_post_ids {
        match blog_post_repository::permanent_delete(pool, id).await {
            Ok(true) => stats.blog_posts_purged += 1,
            Ok(false) => {}
            Err(e) => error!("Failed to permanently delete blog post {}: {}", id, e),
        }
    }

    // Clean up contacts
    let contact_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM contacts WHERE deleted_at IS NOT NULL AND deleted_at < $1",
    )
    .bind(cutoff_date)
    .fetch_all(pool)
    .await?;

    for id in contact_ids {
        match contact_repository::permanent_delete(pool, id).await {
            Ok(true) => stats.contacts_purged += 1,
            Ok(false) => {}
            Err(e) => error!("Failed to permanently delete contact {}: {}", id, e),
        }
    }

    // Clean up event registrations
    let event_registration_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM event_registrations WHERE deleted_at IS NOT NULL AND deleted_at < $1",
    )
    .bind(cutoff_date)
    .fetch_all(pool)
    .await?;

    for id in event_registration_ids {
        match event_registration_repository::permanent_delete(pool, id).await {
            Ok(true) => stats.event_registrations_purged += 1,
            Ok(false) => {}
            Err(e) => error!("Failed to permanently delete event registration {}: {}", id, e),
        }
    }

    // Clean up notifications (requires user_id, so we query it first)
    let notification_data: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, user_id FROM notifications WHERE deleted_at IS NOT NULL AND deleted_at < $1",
    )
    .bind(cutoff_date)
    .fetch_all(pool)
    .await?;

    for (id, user_id) in notification_data {
        match notification_repository::permanent_delete(pool, id, user_id).await {
            Ok(true) => stats.notifications_purged += 1,
            Ok(false) => {}
            Err(e) => error!("Failed to permanently delete notification {}: {}", id, e),
        }
    }

    // Clean up resources
    let resource_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM resources WHERE deleted_at IS NOT NULL AND deleted_at < $1",
    )
    .bind(cutoff_date)
    .fetch_all(pool)
    .await?;

    for id in resource_ids {
        match resource_repository::permanent_delete(pool, id).await {
            Ok(true) => stats.resources_purged += 1,
            Ok(false) => {}
            Err(e) => error!("Failed to permanently delete resource {}: {}", id, e),
        }
    }

    // Clean up visuals
    let visual_ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM visuals WHERE deleted_at IS NOT NULL AND deleted_at < $1",
    )
    .bind(cutoff_date)
    .fetch_all(pool)
    .await?;

    for id in visual_ids {
        match visual_repository::permanent_delete(pool, id).await {
            Ok(true) => stats.visuals_purged += 1,
            Ok(false) => {}
            Err(e) => error!("Failed to permanently delete visual {}: {}", id, e),
        }
    }

    // Calculate total
    stats.total_purged = stats.projects_purged
        + stats.events_purged
        + stats.announcements_purged
        + stats.applications_purged
        + stats.blog_posts_purged
        + stats.contacts_purged
        + stats.event_registrations_purged
        + stats.notifications_purged
        + stats.resources_purged
        + stats.visuals_purged;

    Ok(stats)
}
