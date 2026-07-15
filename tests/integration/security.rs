use axum::http::StatusCode;
use serde_json::json;
use socs_backend::{
    models::event::{EventStatus, EventType},
    repositories::event_repository,
};
use sqlx::PgPool;

use crate::helpers::{
    fixtures::{EventFixture, UserFixture},
    read_body_json, TestApp,
};

#[sqlx::test]
async fn event_management_requires_owner_or_content_approver(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (owner, owner_token) = UserFixture::create_member(&pool).await;
    let (_, other_member_token) = UserFixture::create_member(&pool).await;
    let (_, mentor_token) = UserFixture::create_mentor(&pool).await;

    let owner_event = EventFixture::create(
        &pool,
        "Owner-managed event",
        EventType::Workshop,
        EventStatus::Upcoming,
        Some(owner.id),
    )
    .await;
    let owner_event_uri = format!("/api/events/{}", owner_event.id);

    let response = app.delete(&owner_event_uri, Some(&other_member_token)).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = app.delete(&owner_event_uri, Some(&owner_token)).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(event_repository::find_by_id(&pool, owner_event.id).await.unwrap().is_none());

    let mentor_event = EventFixture::create(
        &pool,
        "Mentor-managed event",
        EventType::Talk,
        EventStatus::Upcoming,
        Some(owner.id),
    )
    .await;
    let mentor_event_uri = format!("/api/events/{}", mentor_event.id);
    let response = app.delete(&mentor_event_uri, Some(&mentor_token)).await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn registrations_and_rich_content_enforce_ownership(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (owner, owner_token) = UserFixture::create_member(&pool).await;
    let (_, other_member_token) = UserFixture::create_member(&pool).await;

    let event = EventFixture::create(
        &pool,
        "Registration access event",
        EventType::Ctf,
        EventStatus::Upcoming,
        Some(owner.id),
    )
    .await;
    let registrations_uri = format!("/api/events/{}/registrations", event.id);
    assert_eq!(
        app.get(&registrations_uri, Some(&other_member_token)).await.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.get(&registrations_uri, Some(&owner_token)).await.status(),
        StatusCode::OK
    );

    let project_payload = json!({
        "title": "Owned project",
        "description": "A project with protected rich content.",
        "tech_stack": ["Rust"],
        "tags": ["security"],
        "featured": false,
    });
    let response = app.post_json("/api/projects", &project_payload, Some(&owner_token)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let project_id = read_body_json(response).await["data"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let feature_payload = json!({"title": "Threat model", "description": "<p>Reviewed</p>"});
    let features_uri = format!("/api/projects/{project_id}/features");
    assert_eq!(
        app.post_json(&features_uri, &feature_payload, Some(&other_member_token)).await.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.post_json(&features_uri, &feature_payload, Some(&owner_token)).await.status(),
        StatusCode::CREATED
    );
}

#[sqlx::test]
async fn privileged_data_and_html_are_protected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (_, member_token) = UserFixture::create_member(&pool).await;
    let (_, mentor_token) = UserFixture::create_mentor(&pool).await;

    assert_eq!(
        app.get("/api/stats/overview", Some(&member_token)).await.status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.get("/api/stats/overview", Some(&mentor_token)).await.status(),
        StatusCode::OK
    );

    let announcement = json!({
        "title": "<script>alert(1)</script>Status",
        "content": "<p>Safe</p><script>alert(1)</script>",
        "category": "general",
        "pinned": false,
    });
    assert_eq!(
        app.post_json("/api/announcements", &announcement, Some(&member_token)).await.status(),
        StatusCode::FORBIDDEN
    );
    let response = app.post_json("/api/announcements", &announcement, Some(&mentor_token)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = read_body_json(response).await;
    assert!(!body["data"]["content"].as_str().unwrap().contains("<script"));
    assert!(!body["data"]["title"].as_str().unwrap().contains("<script"));

    let users = read_body_json(app.get("/api/users", None).await).await;
    for user in users["data"].as_array().unwrap() {
        assert!(user.get("email").is_none(), "public directory must not expose emails");
    }
}
