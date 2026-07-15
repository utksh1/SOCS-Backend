mod config;
mod dto;
mod error;
mod middleware;
mod models;
mod repositories;
mod routes;
mod services;
mod utils;

use axum::{
    http::{header, Method},
    Router,
};
use sqlx::PgPool;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::env::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Config,
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "socs_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // Load config
    let config = Config::from_env().expect("Failed to load configuration");
    
    tracing::info!("Starting SOCS Backend (Rust)");
    
    // Create database pool
    let db = config::database::create_pool(&config.database_url)
        .await
        .expect("Failed to create database pool");
    
    tracing::info!("Database connection established");
    
    // Run migrations
    tracing::info!("Running database migrations...");
    sqlx::migrate!()
        .run(&db)
        .await
        .expect("Failed to run migrations");
    
    tracing::info!("Migrations completed successfully");
    
    let state = AppState {
        db,
        config: config.clone(),
    };
    
    // Build CORS layer
    let cors_origins: Vec<_> = config
        .cors_origin
        .split(',')
        .map(|s| s.trim())
        .filter_map(|origin| origin.parse().ok())
        .collect();
    
    let cors = CorsLayer::new()
        .allow_origin(cors_origins)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(true);
    
    // Auth middleware
    let auth_layer = axum::middleware::from_fn_with_state(state.clone(), middleware::auth::auth_middleware);
    
    // Build auth routes
    let auth_routes = Router::new()
        .route("/register", axum::routing::post(routes::auth::register))
        .route("/login", axum::routing::post(routes::auth::login))
        .route("/me", axum::routing::get(routes::auth::get_me).layer(auth_layer.clone()))
        .route("/update-name", axum::routing::patch(routes::auth::update_name).layer(auth_layer.clone()))
        .route("/change-password", axum::routing::patch(routes::auth::change_password).layer(auth_layer.clone()));
    
    // Build project routes (GET public, POST/PUT/DELETE protected)
    let project_public = Router::new()
        .route("/", axum::routing::get(routes::projects::list_projects))
        .route("/:id", axum::routing::get(routes::projects::get_project));
    
    let project_protected = Router::new()
        .route("/", axum::routing::post(routes::projects::create_project))
        .route("/:id", axum::routing::put(routes::projects::update_project)
            .delete(routes::projects::delete_project))
        .layer(auth_layer.clone());
    
    let project_routes = project_public.merge(project_protected);
    
    // Build event routes (GET public, POST/DELETE protected)
    let event_public = Router::new()
        .route("/", axum::routing::get(routes::events::list_events))
        .route("/:id", axum::routing::get(routes::events::get_event))
        .route("/:id/register", axum::routing::post(routes::event_registrations::register_for_event));
    
    let event_protected = Router::new()
        .route("/", axum::routing::post(routes::events::create_event))
        .route("/:id", axum::routing::delete(routes::events::delete_event))
        .route("/:id/registrations", axum::routing::get(routes::event_registrations::list_registrations))
        .layer(auth_layer.clone());
    
    let event_routes = event_public.merge(event_protected);
    
    // Build team routes (GET public, POST/DELETE protected)
    let team_public = Router::new()
        .route("/", axum::routing::get(routes::team::list_team))
        .route("/slug/:slug", axum::routing::get(routes::team::get_team_member_by_slug))
        .route("/:id", axum::routing::get(routes::team::get_team_member));
    
    let team_protected = Router::new()
        .route("/", axum::routing::post(routes::team::create_team_member))
        .route("/:id", axum::routing::delete(routes::team::delete_team_member))
        .layer(auth_layer.clone());
    
    let team_routes = team_public.merge(team_protected);
    
    // Build resource routes (GET public, POST/DELETE protected)
    let resource_public = Router::new()
        .route("/", axum::routing::get(routes::resources::list_resources))
        .route("/:id", axum::routing::get(routes::resources::get_resource));
    
    let resource_protected = Router::new()
        .route("/", axum::routing::post(routes::resources::create_resource))
        .route("/:id", axum::routing::delete(routes::resources::delete_resource))
        .layer(auth_layer.clone());
    
    let resource_routes = resource_public.merge(resource_protected);
    
    // Build visual routes (GET public, POST/DELETE protected)
    let visual_public = Router::new()
        .route("/", axum::routing::get(routes::visuals::list_visuals))
        .route("/:id", axum::routing::get(routes::visuals::get_visual));
    
    let visual_protected = Router::new()
        .route("/", axum::routing::post(routes::visuals::create_visual))
        .route("/:id", axum::routing::delete(routes::visuals::delete_visual))
        .layer(auth_layer.clone());
    
    let visual_routes = visual_public.merge(visual_protected);
    
    // Build contact routes (POST public, GET protected)
    let contact_public = Router::new()
        .route("/", axum::routing::post(routes::contacts::create_contact));
    
    let contact_protected = Router::new()
        .route("/", axum::routing::get(routes::contacts::list_contacts))
        .layer(auth_layer.clone());
    
    let contact_routes = contact_public.merge(contact_protected);
    
    // Build application routes (POST public, GET/PATCH protected)
    let application_public = Router::new()
        .route("/", axum::routing::post(routes::applications::create_application));
    
    let application_protected = Router::new()
        .route("/", axum::routing::get(routes::applications::list_applications))
        .route("/:id", axum::routing::get(routes::applications::get_application)
            .patch(routes::applications::review_application))
        .layer(auth_layer.clone());
    
    let application_routes = application_public.merge(application_protected);
    
    // Build blog routes (GET public for published, POST/DELETE protected)
    let blog_public = Router::new()
        .route("/", axum::routing::get(routes::blog::list_blog_posts))
        .route("/slug/:slug", axum::routing::get(routes::blog::get_blog_post));
    
    let blog_protected = Router::new()
        .route("/all", axum::routing::get(routes::blog::list_all_blog_posts))
        .route("/", axum::routing::post(routes::blog::create_blog_post))
        .route("/id/:id", axum::routing::delete(routes::blog::delete_blog_post))
        .layer(auth_layer.clone());
    
    let blog_routes = blog_public.merge(blog_protected);
    
    // Build upload routes (all protected - require authentication)
    let upload_routes = Router::new()
        .route("/image", axum::routing::post(routes::upload::upload_image))
        .route("/profile-picture", axum::routing::post(routes::upload::upload_profile_picture))
        .route("/delete", axum::routing::post(routes::upload::delete_image))
        .layer(auth_layer.clone());
    
    // Build user management routes (admin only)
    let user_routes = Router::new()
        .route("/", axum::routing::get(routes::users::list_users)
            .post(routes::users::create_user))
        .route("/:id", axum::routing::get(routes::users::get_user)
            .delete(routes::users::delete_user))
        .route("/:id/role", axum::routing::patch(routes::users::update_user_role))
        .layer(auth_layer.clone());
    
    // Build stats routes (admin only)
    let stats_routes = Router::new()
        .route("/overview", axum::routing::get(routes::stats::get_overview_stats))
        .route("/recent-activity", axum::routing::get(routes::stats::get_recent_activity))
        .route("/users", axum::routing::get(routes::stats::get_user_growth))
        .route("/events", axum::routing::get(routes::stats::get_event_stats))
        .route("/applications", axum::routing::get(routes::stats::get_application_stats))
        .route("/blog", axum::routing::get(routes::stats::get_blog_stats))
        .layer(auth_layer.clone());
    
    // Build notification routes (protected)
    let notification_routes = Router::new()
        .route("/", axum::routing::get(routes::notifications::get_notifications))
        .route("/unread/count", axum::routing::get(routes::notifications::get_unread_count))
        .route("/:id/read", axum::routing::patch(routes::notifications::mark_as_read))
        .route("/read-all", axum::routing::patch(routes::notifications::mark_all_as_read))
        .route("/:id", axum::routing::delete(routes::notifications::delete_notification))
        .layer(auth_layer.clone());
    
    // Build announcement routes (public GET, protected POST/PUT/DELETE)
    let announcement_public = Router::new()
        .route("/", axum::routing::get(routes::notifications::get_announcements))
        .route("/:id", axum::routing::get(routes::notifications::get_announcement));
    
    let announcement_protected = Router::new()
        .route("/", axum::routing::post(routes::notifications::create_announcement))
        .route("/:id", axum::routing::put(routes::notifications::update_announcement)
            .delete(routes::notifications::delete_announcement))
        .route("/:id/pin", axum::routing::patch(routes::notifications::toggle_pin_announcement))
        .layer(auth_layer.clone());
    
    let announcement_routes = Router::new()
        .merge(announcement_public)
        .merge(announcement_protected);
    
    // Build rich content routes (Phase 5)
    let rich_content_routes = Router::new()
        // Project features
        .route("/projects/:project_id/features", 
            axum::routing::get(routes::rich_content::list_project_features)
            .post(routes::rich_content::create_project_feature))
        .route("/projects/:project_id/features/:feature_id",
            axum::routing::put(routes::rich_content::update_project_feature)
            .delete(routes::rich_content::delete_project_feature))
        .route("/projects/:project_id/features/reorder",
            axum::routing::patch(routes::rich_content::reorder_project_features))
        // Project contributors
        .route("/projects/:project_id/contributors",
            axum::routing::get(routes::rich_content::list_project_contributors)
            .post(routes::rich_content::add_project_contributor))
        .route("/projects/:project_id/contributors/:contributor_id",
            axum::routing::delete(routes::rich_content::remove_project_contributor))
        // Event timeline
        .route("/events/:event_id/timeline",
            axum::routing::get(routes::rich_content::list_event_timeline)
            .post(routes::rich_content::create_event_timeline_item))
        .route("/events/:event_id/timeline/:item_id",
            axum::routing::put(routes::rich_content::update_event_timeline_item)
            .delete(routes::rich_content::delete_event_timeline_item))
        .route("/events/:event_id/timeline/reorder",
            axum::routing::patch(routes::rich_content::reorder_event_timeline))
        // Event prerequisites
        .route("/events/:event_id/prerequisites",
            axum::routing::get(routes::rich_content::list_event_prerequisites)
            .post(routes::rich_content::create_event_prerequisite))
        .route("/events/:event_id/prerequisites/:prereq_id",
            axum::routing::put(routes::rich_content::update_event_prerequisite)
            .delete(routes::rich_content::delete_event_prerequisite))
        .route("/events/:event_id/prerequisites/reorder",
            axum::routing::patch(routes::rich_content::reorder_event_prerequisites))
        // Team contributions
        .route("/team/:member_id/contributions",
            axum::routing::get(routes::rich_content::list_team_contributions)
            .post(routes::rich_content::create_team_contribution))
        .route("/team/:member_id/contributions/:contribution_id",
            axum::routing::put(routes::rich_content::update_team_contribution)
            .delete(routes::rich_content::delete_team_contribution))
        .layer(auth_layer.clone());
    
    // Build application router
    let app = Router::new()
        .route("/", axum::routing::get(|| async { "SOCS Backend (Rust)" }))
        .route("/health", axum::routing::get(routes::health::health_check))
        .route("/health/db", axum::routing::get(routes::health::health_check_with_db))
        .nest("/api/auth", auth_routes)
        .nest("/api/projects", project_routes)
        .nest("/api/events", event_routes)
        .nest("/api/team", team_routes)
        .nest("/api/resources", resource_routes)
        .nest("/api/visuals", visual_routes)
        .nest("/api/contacts", contact_routes)
        .nest("/api/applications", application_routes)
        .nest("/api/blog", blog_routes)
        .nest("/api/upload", upload_routes)
        .nest("/api/users", user_routes)
        .nest("/api/stats", stats_routes)
        .nest("/api/notifications", notification_routes)
        .nest("/api/announcements", announcement_routes)
        .nest("/api", rich_content_routes)
        .with_state(state)
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());
    
    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind address");
    
    tracing::info!("🚀 Server listening on {}", addr);
    tracing::info!("📝 API available at http://{}/api", addr);
    tracing::info!("💚 Health check at http://{}/health", addr);
    
    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
