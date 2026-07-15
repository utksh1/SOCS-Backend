use axum::{extract::{State, Query}, Extension, Json};
use serde::Deserialize;
use serde_json::json;

use crate::{
    error::Result,
    models::user::SafeUser,
    AppState,
};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct DateRangeQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub period: Option<String>, // "7d", "30d", "90d", "1y"
}

// Admin only - get overview statistics
pub async fn get_overview_stats(
    State(state): State<AppState>,
    Extension(_admin): Extension<SafeUser>,
) -> Result<Json<serde_json::Value>> {
    // Get user counts
    let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;
    
    let admin_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE role IN ('ADMIN', 'MANAGEMENT')"
    )
    .fetch_one(&state.db)
    .await?;
    
    let new_users_week: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE created_at >= NOW() - INTERVAL '7 days'"
    )
    .fetch_one(&state.db)
    .await?;
    
    // Get project count
    let total_projects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
        .fetch_one(&state.db)
        .await?;
    
    // Get event counts
    let total_events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events")
        .fetch_one(&state.db)
        .await?;
    
    let upcoming_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM events WHERE date >= NOW()"
    )
    .fetch_one(&state.db)
    .await?;
    
    // Get application counts
    let total_applications: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM applications")
        .fetch_one(&state.db)
        .await?;
    
    let pending_applications: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM applications WHERE status = 'PENDING'"
    )
    .fetch_one(&state.db)
    .await?;
    
    // Get blog post count
    let total_blog_posts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blog_posts")
        .fetch_one(&state.db)
        .await?;
    
    let published_posts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM blog_posts WHERE status = 'PUBLISHED'"
    )
    .fetch_one(&state.db)
    .await?;
    
    // Get contact form count
    let total_contacts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contacts")
        .fetch_one(&state.db)
        .await?;
    
    // Get resource count
    let total_resources: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM resources")
        .fetch_one(&state.db)
        .await?;
    
    // Get team member count
    let total_team_members: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM team")
        .fetch_one(&state.db)
        .await?;
    
    Ok(Json(json!({
        "success": true,
        "data": {
            "users": {
                "total": total_users,
                "admins": admin_count,
                "members": total_users - admin_count,
                "new_this_week": new_users_week
            },
            "projects": {
                "total": total_projects
            },
            "events": {
                "total": total_events,
                "upcoming": upcoming_events,
                "past": total_events - upcoming_events
            },
            "applications": {
                "total": total_applications,
                "pending": pending_applications,
                "reviewed": total_applications - pending_applications
            },
            "blog": {
                "total": total_blog_posts,
                "published": published_posts,
                "draft": total_blog_posts - published_posts
            },
            "contacts": {
                "total": total_contacts
            },
            "resources": {
                "total": total_resources
            },
            "team": {
                "total": total_team_members
            }
        }
    })))
}

// Admin only - get recent activity
pub async fn get_recent_activity(
    State(state): State<AppState>,
    Extension(_admin): Extension<SafeUser>,
) -> Result<Json<serde_json::Value>> {
    use sqlx::Row;
    
    // Get recent users (last 10)
    let recent_users = sqlx::query(
        r#"
        SELECT id, name, email, created_at
        FROM users
        ORDER BY created_at DESC
        LIMIT 10
        "#
    )
    .fetch_all(&state.db)
    .await?;
    
    // Get recent applications (last 10)
    let recent_applications = sqlx::query(
        r#"
        SELECT id, name, email, status, created_at
        FROM applications
        ORDER BY created_at DESC
        LIMIT 10
        "#
    )
    .fetch_all(&state.db)
    .await?;
    
    // Get recent events (last 5)
    let recent_events = sqlx::query(
        r#"
        SELECT id, title, date, created_at
        FROM events
        ORDER BY created_at DESC
        LIMIT 5
        "#
    )
    .fetch_all(&state.db)
    .await?;
    
    // Get recent blog posts (last 5)
    let recent_blog = sqlx::query(
        r#"
        SELECT id, title, slug, status, created_at
        FROM blog_posts
        ORDER BY created_at DESC
        LIMIT 5
        "#
    )
    .fetch_all(&state.db)
    .await?;
    
    // Combine into activity feed
    let mut activities = Vec::new();
    
    for user in recent_users {
        let name: String = user.get("name");
        let created_at: chrono::DateTime<chrono::Utc> = user.get("created_at");
        activities.push(json!({
            "type": "user_registered",
            "icon": "user",
            "message": format!("{} registered", name),
            "timestamp": created_at,
            "link": "/admin/users"
        }));
    }
    
    for app in recent_applications {
        let name: String = app.get("name");
        let status: String = app.get("status");
        let created_at: chrono::DateTime<chrono::Utc> = app.get("created_at");
        
        let status_text = match status.as_str() {
            "PENDING" => "submitted an application",
            "APPROVED" => "application was approved",
            "REJECTED" => "application was rejected",
            _ => "updated application"
        };
        activities.push(json!({
            "type": "application",
            "icon": "clipboard",
            "message": format!("{} {}", name, status_text),
            "timestamp": created_at,
            "link": "/admin/applications"
        }));
    }
    
    for event in recent_events {
        let title: String = event.get("title");
        let created_at: chrono::DateTime<chrono::Utc> = event.get("created_at");
        activities.push(json!({
            "type": "event_created",
            "icon": "calendar",
            "message": format!("Event \"{}\" created", title),
            "timestamp": created_at,
            "link": "/events"
        }));
    }
    
    for post in recent_blog {
        let title: String = post.get("title");
        let slug: String = post.get("slug");
        let status: String = post.get("status");
        let created_at: chrono::DateTime<chrono::Utc> = post.get("created_at");
        
        let action = if status == "PUBLISHED" { "published" } else { "drafted" };
        activities.push(json!({
            "type": "blog_post",
            "icon": "file-text",
            "message": format!("Blog post \"{}\" {}", title, action),
            "timestamp": created_at,
            "link": format!("/blog/{}", slug)
        }));
    }
    
    // Sort by timestamp descending
    activities.sort_by(|a, b| {
        let a_time = a["timestamp"].as_str().unwrap_or("");
        let b_time = b["timestamp"].as_str().unwrap_or("");
        b_time.cmp(a_time)
    });
    
    // Take top 20
    activities.truncate(20);
    
    Ok(Json(json!({
        "success": true,
        "data": activities
    })))
}

// Admin only - get user growth statistics
pub async fn get_user_growth(
    State(state): State<AppState>,
    Extension(_admin): Extension<SafeUser>,
    Query(params): Query<DateRangeQuery>,
) -> Result<Json<serde_json::Value>> {
    use sqlx::Row;
    
    // Determine date range
    let days = match params.period.as_deref() {
        Some("7d") => 7,
        Some("30d") => 30,
        Some("90d") => 90,
        Some("1y") => 365,
        _ => 30, // default to 30 days
    };
    
    // Get daily user registration counts
    let growth_data = sqlx::query(
        r#"
        SELECT 
            DATE(created_at) as date,
            COUNT(*) as count
        FROM users
        WHERE created_at >= NOW() - INTERVAL '1 day' * $1
        GROUP BY DATE(created_at)
        ORDER BY date ASC
        "#
    )
    .bind(days)
    .fetch_all(&state.db)
    .await?;
    
    let mut data_points = Vec::new();
    for row in growth_data {
        let date: chrono::NaiveDate = row.get("date");
        let count: i64 = row.get("count");
        data_points.push(json!({
            "date": date.to_string(),
            "count": count
        }));
    }
    
    // Get role distribution
    let role_distribution = sqlx::query(
        r#"
        SELECT 
            role,
            COUNT(*) as count
        FROM users
        GROUP BY role
        ORDER BY count DESC
        "#
    )
    .fetch_all(&state.db)
    .await?;
    
    let mut roles = Vec::new();
    for row in role_distribution {
        let role: String = row.get("role");
        let count: i64 = row.get("count");
        roles.push(json!({
            "role": role,
            "count": count
        }));
    }
    
    Ok(Json(json!({
        "success": true,
        "data": {
            "growth": data_points,
            "roles": roles,
            "period": format!("{}d", days)
        }
    })))
}

// Admin only - get event statistics
pub async fn get_event_stats(
    State(state): State<AppState>,
    Extension(_admin): Extension<SafeUser>,
    Query(params): Query<DateRangeQuery>,
) -> Result<Json<serde_json::Value>> {
    use sqlx::Row;
    
    let days = match params.period.as_deref() {
        Some("7d") => 7,
        Some("30d") => 30,
        Some("90d") => 90,
        Some("1y") => 365,
        _ => 30,
    };
    
    // Get event registration counts
    let event_registrations = sqlx::query(
        r#"
        SELECT 
            e.id,
            e.title,
            e.date,
            e.event_type,
            COUNT(r.id) as registration_count
        FROM events e
        LEFT JOIN event_registrations r ON e.id = r.event_id
        WHERE e.created_at >= NOW() - INTERVAL '1 day' * $1
        GROUP BY e.id, e.title, e.date, e.event_type
        ORDER BY e.date DESC
        "#
    )
    .bind(days)
    .fetch_all(&state.db)
    .await?;
    
    let mut events = Vec::new();
    for row in event_registrations {
        let id: uuid::Uuid = row.get("id");
        let title: String = row.get("title");
        let date: chrono::DateTime<chrono::Utc> = row.get("date");
        let event_type: String = row.get("event_type");
        let registration_count: i64 = row.get("registration_count");
        
        events.push(json!({
            "id": id,
            "title": title,
            "date": date,
            "type": event_type,
            "registrations": registration_count
        }));
    }
    
    // Get event type distribution
    let type_distribution = sqlx::query(
        r#"
        SELECT 
            event_type,
            COUNT(*) as count
        FROM events
        WHERE created_at >= NOW() - INTERVAL '1 day' * $1
        GROUP BY event_type
        ORDER BY count DESC
        "#
    )
    .bind(days)
    .fetch_all(&state.db)
    .await?;
    
    let mut types = Vec::new();
    for row in type_distribution {
        let event_type: String = row.get("event_type");
        let count: i64 = row.get("count");
        types.push(json!({
            "type": event_type,
            "count": count
        }));
    }
    
    Ok(Json(json!({
        "success": true,
        "data": {
            "events": events,
            "type_distribution": types,
            "period": format!("{}d", days)
        }
    })))
}

// Admin only - get application statistics
pub async fn get_application_stats(
    State(state): State<AppState>,
    Extension(_admin): Extension<SafeUser>,
    Query(params): Query<DateRangeQuery>,
) -> Result<Json<serde_json::Value>> {
    use sqlx::Row;
    
    let days = match params.period.as_deref() {
        Some("7d") => 7,
        Some("30d") => 30,
        Some("90d") => 90,
        Some("1y") => 365,
        _ => 30,
    };
    
    // Get application timeline
    let timeline = sqlx::query(
        r#"
        SELECT 
            DATE(created_at) as date,
            status,
            COUNT(*) as count
        FROM applications
        WHERE created_at >= NOW() - INTERVAL '1 day' * $1
        GROUP BY DATE(created_at), status
        ORDER BY date ASC
        "#
    )
    .bind(days)
    .fetch_all(&state.db)
    .await?;
    
    let mut timeline_data = Vec::new();
    for row in timeline {
        let date: chrono::NaiveDate = row.get("date");
        let status: String = row.get("status");
        let count: i64 = row.get("count");
        timeline_data.push(json!({
            "date": date.to_string(),
            "status": status,
            "count": count
        }));
    }
    
    // Get status distribution
    let status_distribution = sqlx::query(
        r#"
        SELECT 
            status,
            COUNT(*) as count
        FROM applications
        WHERE created_at >= NOW() - INTERVAL '1 day' * $1
        GROUP BY status
        "#
    )
    .bind(days)
    .fetch_all(&state.db)
    .await?;
    
    let mut statuses = Vec::new();
    for row in status_distribution {
        let status: String = row.get("status");
        let count: i64 = row.get("count");
        statuses.push(json!({
            "status": status,
            "count": count
        }));
    }
    
    // Calculate approval rate
    let total: i64 = statuses.iter()
        .map(|s| s["count"].as_i64().unwrap_or(0))
        .sum();
    
    let approved = statuses.iter()
        .find(|s| s["status"].as_str() == Some("APPROVED"))
        .and_then(|s| s["count"].as_i64())
        .unwrap_or(0);
    
    let approval_rate = if total > 0 {
        (approved as f64 / total as f64 * 100.0).round()
    } else {
        0.0
    };
    
    Ok(Json(json!({
        "success": true,
        "data": {
            "timeline": timeline_data,
            "status_distribution": statuses,
            "approval_rate": approval_rate,
            "total": total,
            "period": format!("{}d", days)
        }
    })))
}

// Admin only - get blog engagement statistics
pub async fn get_blog_stats(
    State(state): State<AppState>,
    Extension(_admin): Extension<SafeUser>,
    Query(params): Query<DateRangeQuery>,
) -> Result<Json<serde_json::Value>> {
    use sqlx::Row;
    
    let days = match params.period.as_deref() {
        Some("7d") => 7,
        Some("30d") => 30,
        Some("90d") => 90,
        Some("1y") => 365,
        _ => 30,
    };
    
    // Get blog post timeline
    let timeline = sqlx::query(
        r#"
        SELECT 
            DATE(created_at) as date,
            COUNT(*) as count
        FROM blog_posts
        WHERE created_at >= NOW() - INTERVAL '1 day' * $1
        GROUP BY DATE(created_at)
        ORDER BY date ASC
        "#
    )
    .bind(days)
    .fetch_all(&state.db)
    .await?;
    
    let mut timeline_data = Vec::new();
    for row in timeline {
        let date: chrono::NaiveDate = row.get("date");
        let count: i64 = row.get("count");
        timeline_data.push(json!({
            "date": date.to_string(),
            "count": count
        }));
    }
    
    // Get status distribution
    let status_distribution = sqlx::query(
        r#"
        SELECT 
            status,
            COUNT(*) as count
        FROM blog_posts
        WHERE created_at >= NOW() - INTERVAL '1 day' * $1
        GROUP BY status
        "#
    )
    .bind(days)
    .fetch_all(&state.db)
    .await?;
    
    let mut statuses = Vec::new();
    for row in status_distribution {
        let status: String = row.get("status");
        let count: i64 = row.get("count");
        statuses.push(json!({
            "status": status,
            "count": count
        }));
    }
    
    // Get most recent posts
    let recent_posts = sqlx::query(
        r#"
        SELECT 
            id,
            title,
            slug,
            status,
            created_at
        FROM blog_posts
        WHERE created_at >= NOW() - INTERVAL '1 day' * $1
        ORDER BY created_at DESC
        LIMIT 10
        "#
    )
    .bind(days)
    .fetch_all(&state.db)
    .await?;
    
    let mut posts = Vec::new();
    for row in recent_posts {
        let id: uuid::Uuid = row.get("id");
        let title: String = row.get("title");
        let slug: String = row.get("slug");
        let status: String = row.get("status");
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        
        posts.push(json!({
            "id": id,
            "title": title,
            "slug": slug,
            "status": status,
            "created_at": created_at
        }));
    }
    
    Ok(Json(json!({
        "success": true,
        "data": {
            "timeline": timeline_data,
            "status_distribution": statuses,
            "recent_posts": posts,
            "period": format!("{}d", days)
        }
    })))
}
