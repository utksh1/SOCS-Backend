use axum::http::StatusCode;
use sqlx::PgPool;

use crate::helpers::TestApp;

#[sqlx::test]
async fn test_public_endpoints(pool: PgPool) {
    let app = TestApp::new(pool).await;

    // Test health check
    let res = app.get("/health", None).await;
    assert_eq!(res.status(), StatusCode::OK);

    // Test API root
    let res = app.get("/", None).await;
    assert_eq!(res.status(), StatusCode::OK);

    // Test events list (public)
    let res = app.get("/api/events", None).await;
    assert_eq!(res.status(), StatusCode::OK);

    // Test projects list (public)
    let res = app.get("/api/projects", None).await;
    assert_eq!(res.status(), StatusCode::OK);

    // Test users list (public)
    let res = app.get("/api/users", None).await;
    assert_eq!(res.status(), StatusCode::OK);
}
