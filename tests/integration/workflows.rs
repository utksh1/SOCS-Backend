use axum::http::StatusCode;
use sqlx::PgPool;

use crate::helpers::{TestApp, fixtures::UserFixture};

#[sqlx::test]
async fn test_project_submission_and_approval_workflow(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    
    let (_member, member_token) = UserFixture::create_member(&pool).await;
    let (_, mentor_token) = UserFixture::create_mentor(&pool).await;
    
    // 1. Member submits a new project
    let create_payload = serde_json::json!({
        "title": "New AI Project",
        "description": "An ambitious AI project",
        "tech_stack": ["Rust"],
        "slug": "new-ai-project",
        "github_link": null,
        "tags": ["AI"],
        "featured": false
    });
    
    let res = app.post_json("/api/projects", &create_payload, Some(&member_token)).await;
    assert_eq!(res.status(), StatusCode::CREATED);
    
    let body = crate::helpers::read_body_json(res).await;
    let project_id = body["data"]["id"].as_str().unwrap();
    
    // 2. Member tries to approve it themselves (should fail)
    let approve_payload = serde_json::json!({
        "status": "approved",
    });
    let approve_uri = format!("/api/projects/{}/approve", project_id);
    let res = app.post_json(&approve_uri, &approve_payload, Some(&member_token)).await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    
    // 3. Mentor approves the project
    let res = app.post_json(&approve_uri, &approve_payload, Some(&mentor_token)).await;
    assert_eq!(res.status(), StatusCode::OK);
    
    // 4. Project is now visible in the public list
    let res = app.get("/api/projects", None).await;
    let body = crate::helpers::read_body_json(res).await;
    
    let projects = body["data"].as_array().unwrap();
    let found = projects.iter().any(|p| p["id"] == project_id);
    assert!(found, "Approved project should be visible in public list");
}
