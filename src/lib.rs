// Library exports for testing
pub mod config;
pub mod dto;
pub mod error;
pub mod middleware;
pub mod models;
pub mod repositories;
pub mod routes;
pub mod services;
pub mod tasks;
pub mod telemetry;
pub mod utils;

use axum::{
    http::{header, Method},
    Router,
};
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::TraceLayer,
};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: config::env::Config,
    /// Redis is required in production for rate limiting. It is optional only
    /// for hermetic test routers where rate limiting is explicitly disabled.
    pub redis: Option<ConnectionManager>,
}

/// Build the application router with the given state
pub fn build_router(state: AppState) -> Router {
    // Build CORS layer — validate configured origins at startup.
    let cors_origins: Vec<_> = state.config
        .cors_origin
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .filter_map(|origin| match origin.parse::<axum::http::HeaderValue>() {
            Ok(value) => Some(value),
            Err(error) => {
                tracing::error!(%error, origin, "Ignoring malformed CORS origin");
                None
            }
        })
        .collect();

    if cors_origins.is_empty() {
        panic!(
            "CORS_ORIGIN produced zero valid origins (raw value: {:?}). \
             Fix the value or the server will reject every cross-origin request.",
            state.config.cors_origin
        );
    }
    
    let cors = CorsLayer::new()
        .allow_origin(cors_origins)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(true);
    
    // Auth middleware
    let auth_layer = axum::middleware::from_fn_with_state(state.clone(), middleware::auth::auth_middleware);
    let optional_auth_layer = axum::middleware::from_fn_with_state(state.clone(), middleware::auth::optional_auth_middleware);
    let toplead_layer = axum::middleware::from_fn_with_state(state.clone(), middleware::auth::require_toplead);
    let verification_layer = axum::middleware::from_fn(middleware::verification::require_verified_email);
    
    // Rate limit middleware
    let auth_rate_limit = axum::middleware::from_fn_with_state(state.clone(), middleware::rate_limit::auth_rate_limit_middleware);
    let public_rate_limit = axum::middleware::from_fn_with_state(state.clone(), middleware::rate_limit::public_rate_limit_middleware);
    let authenticated_rate_limit = axum::middleware::from_fn_with_state(state.clone(), middleware::rate_limit::authenticated_rate_limit_middleware);
    let upload_rate_limit = axum::middleware::from_fn_with_state(state.clone(), middleware::rate_limit::upload_rate_limit_middleware);
    
    // Build auth routes
    let auth_register = Router::new()
        .route("/register", axum::routing::post(routes::auth::register))
        .layer(toplead_layer.clone())
        .layer(auth_rate_limit.clone());
    
    let auth_public = Router::new()
        .route("/login", axum::routing::post(routes::auth::login))
        .route("/verify-email", axum::routing::get(routes::auth::verify_email))
        .route("/forgot-password", axum::routing::post(routes::auth::forgot_password))
        .route("/reset-password", axum::routing::post(routes::auth::reset_password))
        .layer(auth_rate_limit.clone());
    
    let auth_change_password = Router::new()
        .route("/change-password", axum::routing::patch(routes::auth::change_password))
        .layer(verification_layer.clone())
        .layer(auth_layer.clone());

    let auth_resend = Router::new()
        .route("/resend-verification", axum::routing::post(routes::auth::resend_verification))
        .layer(auth_layer.clone());
    
    let auth_protected = Router::new()
        .route("/me", axum::routing::get(routes::auth::get_me))
        .route(
            "/update-name", 
            axum::routing::patch(routes::auth::update_name)
                .route_layer(verification_layer.clone())
        )
        .layer(authenticated_rate_limit.clone())
        .layer(auth_layer.clone());
    
    let auth_routes = auth_register
        .merge(auth_public)
        .merge(auth_change_password.layer(auth_rate_limit.clone()))
        .merge(auth_resend.layer(auth_rate_limit.clone()))
        .merge(auth_protected);
    
    // Build project routes
    let project_public = Router::new()
        .route("/", axum::routing::get(routes::projects::list_projects))
        .route("/:id", axum::routing::get(routes::projects::get_project))
        .layer(public_rate_limit.clone())
        .layer(optional_auth_layer.clone());
    
    let project_protected = Router::new()
        .route("/", axum::routing::post(routes::projects::create_project))
        .route("/all", axum::routing::get(routes::projects::list_all_projects))
        .route("/my", axum::routing::get(routes::projects::list_my_projects))
        .route("/:id", axum::routing::put(routes::projects::update_project)
            .delete(routes::projects::delete_project))
        .route("/:id/approve", axum::routing::post(routes::projects::approve_or_reject_project))
        .route("/:id/collaborators", axum::routing::post(routes::projects::add_collaborator))
        .route("/:id/collaborators/:user_id", axum::routing::delete(routes::projects::remove_collaborator))
        .layer(authenticated_rate_limit.clone())
        .layer(auth_layer.clone());
    
    let project_routes = project_public.merge(project_protected);
    
    // Build event routes
    let event_public = Router::new()
        .route("/", axum::routing::get(routes::events::list_events))
        .route("/:id", axum::routing::get(routes::events::get_event))
        .route("/:id/register", axum::routing::post(routes::event_registrations::register_for_event))
        .layer(public_rate_limit.clone());
    
    let event_protected = Router::new()
        .route("/", axum::routing::post(routes::events::create_event))
        .route("/:id", axum::routing::delete(routes::events::delete_event))
        .route("/:id/registrations", axum::routing::get(routes::event_registrations::list_registrations))
        .layer(authenticated_rate_limit.clone())
        .layer(auth_layer.clone());
    
    let event_routes = event_public.merge(event_protected);
    
    // Build user routes
    let user_public = Router::new()
        .route("/", axum::routing::get(routes::users::list_users))
        .route("/slug/:slug", axum::routing::get(routes::users::get_user_by_slug))
        .route("/:id", axum::routing::get(routes::users::get_user))
        .layer(public_rate_limit.clone());
    
    let user_protected = Router::new()
        .route("/", axum::routing::post(routes::users::create_user))
        .route("/:id", axum::routing::put(routes::users::update_user))
        .layer(authenticated_rate_limit.clone())
        .layer(auth_layer.clone());

    let user_critical = Router::new()
        .route("/:id", axum::routing::delete(routes::users::delete_user))
        .layer(authenticated_rate_limit.clone())
        .layer(auth_layer.clone());
    
    let user_routes = user_public.merge(user_protected).merge(user_critical);
    
    // Build resource routes
    let resource_public = Router::new()
        .route("/", axum::routing::get(routes::resources::list_resources))
        .route("/:id", axum::routing::get(routes::resources::get_resource))
        .layer(public_rate_limit.clone())
        .layer(optional_auth_layer.clone());
    
    let resource_protected = Router::new()
        .route("/", axum::routing::post(routes::resources::create_resource))
        .route("/all", axum::routing::get(routes::resources::list_all_resources))
        .route("/my", axum::routing::get(routes::resources::list_my_resources))
        .route("/:id", axum::routing::put(routes::resources::update_resource)
            .delete(routes::resources::delete_resource))
        .route("/:id/approve", axum::routing::post(routes::resources::approve_or_reject_resource))
        .route("/:id/collaborators", axum::routing::post(routes::resources::add_collaborator))
        .route("/:id/collaborators/:user_id", axum::routing::delete(routes::resources::remove_collaborator))
        .layer(verification_layer.clone())
        .layer(authenticated_rate_limit.clone())
        .layer(auth_layer.clone());
    
    let resource_routes = resource_public.merge(resource_protected);
    
    // Build visual routes
    let visual_public = Router::new()
        .route("/", axum::routing::get(routes::visuals::list_visuals))
        .route("/:id", axum::routing::get(routes::visuals::get_visual))
        .layer(public_rate_limit.clone());
    
    let visual_protected = Router::new()
        .route("/", axum::routing::post(routes::visuals::create_visual))
        .route("/:id", axum::routing::delete(routes::visuals::delete_visual))
        .layer(verification_layer.clone())
        .layer(authenticated_rate_limit.clone())
        .layer(auth_layer.clone());
    
    let visual_routes = visual_public.merge(visual_protected);
    
    // Build contact routes
    let contact_public = Router::new()
        .route("/", axum::routing::post(routes::contacts::create_contact))
        .layer(public_rate_limit.clone());
    
    let contact_protected = Router::new()
        .route("/", axum::routing::get(routes::contacts::list_contacts))
        .layer(verification_layer.clone())
        .layer(authenticated_rate_limit.clone())
        .layer(auth_layer.clone());
    
    let contact_routes = contact_public.merge(contact_protected);
    
    // Build application routes
    let application_public = Router::new()
        .route("/", axum::routing::post(routes::applications::create_application))
        .layer(public_rate_limit.clone());
    
    let application_protected = Router::new()
        .route("/", axum::routing::get(routes::applications::list_applications))
        .route("/:id", axum::routing::get(routes::applications::get_application)
            .patch(routes::applications::review_application))
        .layer(verification_layer.clone())
        .layer(authenticated_rate_limit.clone())
        .layer(auth_layer.clone());
    
    let application_routes = application_public.merge(application_protected);
    
    // Build blog routes
    let blog_public = Router::new()
        .route("/", axum::routing::get(routes::blog::list_blog_posts))
        .route("/slug/:slug", axum::routing::get(routes::blog::get_blog_post))
        .layer(public_rate_limit.clone())
        .layer(optional_auth_layer.clone());
    
    let blog_protected = Router::new()
        .route("/all", axum::routing::get(routes::blog::list_all_blog_posts))
        .route("/my", axum::routing::get(routes::blog::list_my_blog_posts))
        .route("/", axum::routing::post(routes::blog::create_blog_post))
        .route("/:id", axum::routing::put(routes::blog::update_blog_post)
            .delete(routes::blog::delete_blog_post))
        .route("/:id/approve", axum::routing::post(routes::blog::approve_or_reject_blog_post))
        .route("/:id/collaborators", axum::routing::post(routes::blog::add_collaborator))
        .route("/:id/collaborators/:user_id", axum::routing::delete(routes::blog::remove_collaborator))
        .layer(verification_layer.clone())
        .layer(authenticated_rate_limit.clone())
        .layer(auth_layer.clone());
    
    let blog_routes = blog_public.merge(blog_protected);
    
    // Build upload routes
    let upload_routes = Router::new()
        .route("/image", axum::routing::post(routes::upload::upload_image))
        .route("/profile-picture", axum::routing::post(routes::upload::upload_profile_picture))
        .route("/delete", axum::routing::post(routes::upload::delete_image))
        .layer(routes::upload::upload_body_limit())
        .layer(upload_rate_limit)
        .layer(verification_layer.clone())
        .layer(authenticated_rate_limit.clone())
        .layer(auth_layer.clone());
    
    // Build stats routes
    let stats_routes = Router::new()
        .route("/overview", axum::routing::get(routes::stats::get_overview_stats))
        .route("/recent-activity", axum::routing::get(routes::stats::get_recent_activity))
        .route("/users", axum::routing::get(routes::stats::get_user_growth))
        .route("/events", axum::routing::get(routes::stats::get_event_stats))
        .route("/applications", axum::routing::get(routes::stats::get_application_stats))
        .route("/blog", axum::routing::get(routes::stats::get_blog_stats))
        .layer(verification_layer.clone())
        .layer(authenticated_rate_limit.clone())
        .layer(auth_layer.clone());
    
    // Build notification routes
    let notification_routes = Router::new()
        .route("/", axum::routing::get(routes::notifications::get_notifications))
        .route("/unread/count", axum::routing::get(routes::notifications::get_unread_count))
        .route("/:id/read", axum::routing::patch(routes::notifications::mark_as_read))
        .route("/read-all", axum::routing::patch(routes::notifications::mark_all_as_read))
        .route("/:id", axum::routing::delete(routes::notifications::delete_notification))
        .layer(verification_layer.clone())
        .layer(authenticated_rate_limit.clone())
        .layer(auth_layer.clone());
    
    // Build announcement routes
    let announcement_public = Router::new()
        .route("/", axum::routing::get(routes::notifications::get_announcements))
        .route("/:id", axum::routing::get(routes::notifications::get_announcement))
        .layer(public_rate_limit.clone());
    
    let announcement_protected = Router::new()
        .route("/", axum::routing::post(routes::notifications::create_announcement))
        .route("/:id", axum::routing::put(routes::notifications::update_announcement)
            .delete(routes::notifications::delete_announcement))
        .route("/:id/pin", axum::routing::patch(routes::notifications::toggle_pin_announcement))
        .layer(verification_layer.clone())
        .layer(authenticated_rate_limit.clone())
        .layer(auth_layer.clone());
    
    let announcement_routes = Router::new()
        .merge(announcement_public)
        .merge(announcement_protected);
    
    // Build rich content routes — read endpoints use optional auth so
    // unauthenticated visitors can read content for approved parents.
    let rich_content_public = Router::new()
        .route("/projects/:project_id/features",
            axum::routing::get(routes::rich_content::list_project_features))
        .route("/projects/:project_id/contributors",
            axum::routing::get(routes::rich_content::list_project_contributors))
        .route("/events/:event_id/timeline",
            axum::routing::get(routes::rich_content::list_event_timeline))
        .route("/events/:event_id/prerequisites",
            axum::routing::get(routes::rich_content::list_event_prerequisites))
        .route("/users/:user_id/contributions",
            axum::routing::get(routes::rich_content::list_team_contributions))
        .layer(public_rate_limit.clone())
        .layer(optional_auth_layer.clone());

    let rich_content_protected = Router::new()
        .route("/projects/:project_id/features",
            axum::routing::post(routes::rich_content::create_project_feature))
        .route("/projects/:project_id/features/:feature_id",
            axum::routing::put(routes::rich_content::update_project_feature)
            .delete(routes::rich_content::delete_project_feature))
        .route("/projects/:project_id/features/reorder",
            axum::routing::patch(routes::rich_content::reorder_project_features))
        .route("/projects/:project_id/contributors",
            axum::routing::post(routes::rich_content::add_project_contributor))
        .route("/projects/:project_id/contributors/:contributor_id",
            axum::routing::delete(routes::rich_content::remove_project_contributor))
        .route("/events/:event_id/timeline",
            axum::routing::post(routes::rich_content::create_event_timeline_item))
        .route("/events/:event_id/timeline/:item_id",
            axum::routing::put(routes::rich_content::update_event_timeline_item)
            .delete(routes::rich_content::delete_event_timeline_item))
        .route("/events/:event_id/timeline/reorder",
            axum::routing::patch(routes::rich_content::reorder_event_timeline))
        .route("/events/:event_id/prerequisites",
            axum::routing::post(routes::rich_content::create_event_prerequisite))
        .route("/events/:event_id/prerequisites/:prereq_id",
            axum::routing::put(routes::rich_content::update_event_prerequisite)
            .delete(routes::rich_content::delete_event_prerequisite))
        .route("/events/:event_id/prerequisites/reorder",
            axum::routing::patch(routes::rich_content::reorder_event_prerequisites))
        .route("/users/:user_id/contributions",
            axum::routing::post(routes::rich_content::create_team_contribution))
        .route("/users/:user_id/contributions/:contribution_id",
            axum::routing::put(routes::rich_content::update_team_contribution)
            .delete(routes::rich_content::delete_team_contribution))
        .layer(verification_layer.clone())
        .layer(authenticated_rate_limit.clone())
        .layer(auth_layer.clone());

    let rich_content_routes = rich_content_public.merge(rich_content_protected);
    
    // Build admin routes
    let admin_routes = routes::admin_routes::configure()
        .layer(verification_layer.clone())
        .layer(authenticated_rate_limit.clone())
        .layer(auth_layer.clone());

    // Root / health routes get the public rate-limit bucket so the
    // DB-writing /health/db endpoint is not trivially abusable.
    let root_routes = Router::new()
        .route("/", axum::routing::get(|| async { "SOCS Backend (Rust)" }))
        .route("/health", axum::routing::get(routes::health::health_check))
        .route("/health/db", axum::routing::get(routes::health::health_check_with_db))
        .layer(public_rate_limit.clone());
    
    // Build application router
    Router::new()
        .merge(root_routes)
        .nest("/api/auth", auth_routes)
        .nest("/api/projects", project_routes)
        .nest("/api/events", event_routes)
        .nest("/api/users", user_routes)
        .nest("/api/resources", resource_routes)
        .nest("/api/visuals", visual_routes)
        .nest("/api/contacts", contact_routes)
        .nest("/api/applications", application_routes)
        .nest("/api/blog", blog_routes)
        .nest("/api/upload", upload_routes)
        .nest("/api/stats", stats_routes)
        .nest("/api/notifications", notification_routes)
        .nest("/api/announcements", announcement_routes)
        .nest("/api/admin", admin_routes)
        .nest("/api", rich_content_routes)
        .with_state(state)
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<_>| {
            tracing::info_span!(
                "http_request",
                method = %request.method(),
                path = request.uri().path(),
            )
        }))
        .layer(axum::middleware::from_fn(middleware::request_logging::log_request))
}
