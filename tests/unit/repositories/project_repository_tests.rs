use socs_backend::repositories::{project_repository, user_repository};
use socs_backend::models::project::{Project, ContentStatus};
use sqlx::PgPool;
use uuid::Uuid;

async fn insert_test_project(pool: &PgPool, title: &str, user_id: Uuid) -> Project {
    let slug = format!("{}-{}", title.to_lowercase().replace(" ", "-"), Uuid::new_v4());
    
    sqlx::query_as::<_, Project>(
        r#"
        INSERT INTO projects (slug, title, description, tech_stack, github_link, tags, featured, status, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(&slug)
    .bind(title)
    .bind("A test project description")
    .bind(vec!["Rust".to_string(), "PostgreSQL".to_string()])
    .bind("https://github.com/test/repo")
    .bind(vec!["backend".to_string()])
    .bind(false)
    .bind(ContentStatus::Pending)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("Failed to insert test project")
}

#[sqlx::test]
async fn test_find_by_id(pool: PgPool) {
    let email = format!("proj_user_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Project User", &email, "hash").await.unwrap();
    let project = insert_test_project(&pool, "Test Project ID", user.id).await;

    let result = project_repository::find_by_id(&pool, project.id).await;
    assert!(result.is_ok());
    
    let found = result.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().title, "Test Project ID");
}

#[sqlx::test]
async fn test_find_by_slug(pool: PgPool) {
    let email = format!("proj_user_slug_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Project User Slug", &email, "hash").await.unwrap();
    let project = insert_test_project(&pool, "Test Project Slug", user.id).await;

    let result = project_repository::find_by_slug(&pool, &project.slug).await;
    assert!(result.is_ok());
    
    let found = result.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, project.id);
}

#[sqlx::test]
async fn test_update(pool: PgPool) {
    let email = format!("proj_user_update_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Project User Update", &email, "hash").await.unwrap();
    let project = insert_test_project(&pool, "Project To Update", user.id).await;

    let new_title = "Updated Project Title";
    let new_slug = format!("updated-slug-{}", Uuid::new_v4());
    let new_tech = vec!["Go".to_string(), "Redis".to_string()];
    
    let result = project_repository::update(
        &pool,
        project.id,
        Some(&new_slug),
        Some(new_title),
        None,
        Some(&new_tech),
        None,
        None,
        Some(true)
    ).await;
    
    assert!(result.is_ok());
    let updated = result.unwrap();
    
    assert_eq!(updated.title, new_title);
    assert_eq!(updated.slug, new_slug);
    assert_eq!(updated.tech_stack, new_tech);
    assert_eq!(updated.featured, true);
    // Unchanged fields remain same
    assert_eq!(updated.description, project.description);
}

#[sqlx::test]
async fn test_soft_delete_and_restore(pool: PgPool) {
    let email = format!("proj_user_del_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Project User Delete", &email, "hash").await.unwrap();
    let project = insert_test_project(&pool, "Project To Delete", user.id).await;

    // Soft delete via delete() which calls soft_delete()
    let deleted = project_repository::delete(&pool, project.id).await.unwrap();
    assert!(deleted);

    // Should not be found via normal find methods
    let found = project_repository::find_by_id(&pool, project.id).await.unwrap();
    assert!(found.is_none());
    
    let found_slug = project_repository::find_by_slug(&pool, &project.slug).await.unwrap();
    assert!(found_slug.is_none());

    // Should be found via with_deleted method
    let found_deleted = project_repository::find_by_id_with_deleted(&pool, project.id).await.unwrap();
    assert!(found_deleted.is_some());

    // Restore
    let restored = project_repository::restore(&pool, project.id).await.unwrap();
    assert!(restored);

    // Should be found again
    let found_restored = project_repository::find_by_id(&pool, project.id).await.unwrap();
    assert!(found_restored.is_some());
}

#[sqlx::test]
async fn test_permanent_delete(pool: PgPool) {
    let email = format!("proj_user_perm_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Project User Perm", &email, "hash").await.unwrap();
    let project = insert_test_project(&pool, "Project To Perm Delete", user.id).await;

    project_repository::soft_delete(&pool, project.id).await.unwrap();
    let deleted = project_repository::permanent_delete(&pool, project.id).await.unwrap();
    assert!(deleted);

    // Should not be found even with deleted
    let found = project_repository::find_by_id_with_deleted(&pool, project.id).await.unwrap();
    assert!(found.is_none());
}
