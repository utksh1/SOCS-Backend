use axum::http::StatusCode;
use sqlx::PgPool;

use crate::helpers::{TestApp, fixtures::UserFixture};

#[sqlx::test]
async fn test_ownership_permissions(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    
    // Create two members
    let (user1, token1) = UserFixture::create_member(&pool).await;
    let (user2, token2) = UserFixture::create_member(&pool).await;
    
    let payload = serde_json::json!({
        "name": "Updated Name"
    });
    
    // User1 can update User1's own profile
    let uri1 = format!("/api/users/{}", user1.id);
    let res = app.put_json(&uri1, &payload, Some(&token1)).await;
    if res.status() != StatusCode::OK {
        let body_bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        println!("ERROR RESPONSE: {:?}", String::from_utf8_lossy(&body_bytes));
        panic!("Status was not OK");
    }
    
    // User1 CANNOT update User2's profile
    let uri2 = format!("/api/users/{}", user2.id);
    let res = app.put_json(&uri2, &payload, Some(&token1)).await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    
    // User2 can update User2's own profile
    let res = app.put_json(&uri2, &payload, Some(&token2)).await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_role_hierarchy_permissions(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    
    let (toplead, toplead_token) = UserFixture::create_toplead(&pool).await;
    let (mentor, mentor_token) = UserFixture::create_mentor(&pool).await;
    let (member, _) = UserFixture::create_member(&pool).await;
    
    let payload = serde_json::json!({
        "name": "Updated Name"
    });
    
    let member_uri = format!("/api/users/{}", member.id);
    let mentor_uri = format!("/api/users/{}", mentor.id);
    
    // Mentor can update Member (Level 4 > Level 1)
    let res = app.put_json(&member_uri, &payload, Some(&mentor_token)).await;
    assert_eq!(res.status(), StatusCode::OK);
    
    // Mentor CANNOT update TopLead (Level 4 <= Level 5)
    let toplead_uri = format!("/api/users/{}", toplead.id);
    let res = app.put_json(&toplead_uri, &payload, Some(&mentor_token)).await;
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    
    // TopLead can update Mentor (Level 5 > Level 4)
    let res = app.put_json(&mentor_uri, &payload, Some(&toplead_token)).await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_cannot_escalate_own_role(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (member, token) = UserFixture::create_member(&pool).await;
    
    // Member tries to give themselves TopLead role
    let payload = serde_json::json!({
        "roles": ["TopLead", "Member"]
    });
    
    let uri = format!("/api/users/{}", member.id);
    let res = app.put_json(&uri, &payload, Some(&token)).await;
    
    // Should be Bad Request (cannot change own roles)
    assert!(res.status() == StatusCode::BAD_REQUEST || res.status() == StatusCode::FORBIDDEN);
}
