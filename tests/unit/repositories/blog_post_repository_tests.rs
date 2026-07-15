use socs_backend::repositories::{blog_post_repository, user_repository};
use socs_backend::models::blog_post::{BlogPost, PostCategory, PostStatus, ContentStatus};
use sqlx::PgPool;
use uuid::Uuid;

async fn insert_test_blog_post(pool: &PgPool, title: &str, user_id: Uuid) -> BlogPost {
    let slug = format!("{}-{}", title.to_lowercase().replace(" ", "-"), Uuid::new_v4());
    
    sqlx::query_as::<_, BlogPost>(
        r#"
        INSERT INTO blog_posts (slug, title, excerpt, content, category, status, approval_status, tags, author_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#,
    )
    .bind(&slug)
    .bind(title)
    .bind("An excerpt")
    .bind("The full content of the blog post")
    .bind(PostCategory::Tutorial)
    .bind(PostStatus::Draft)
    .bind(ContentStatus::Pending)
    .bind(vec!["rust".to_string()])
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("Failed to insert test blog post")
}

#[sqlx::test]
async fn test_find_by_slug(pool: PgPool) {
    let email = format!("blog_user_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Blog User", &email, "hash").await.unwrap();
    let post = insert_test_blog_post(&pool, "Test Blog Post", user.id).await;

    let result = blog_post_repository::find_by_slug(&pool, &post.slug).await;
    assert!(result.is_ok());
    
    let found = result.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, post.id);
}

#[sqlx::test]
async fn test_soft_delete_and_restore(pool: PgPool) {
    let email = format!("blog_del_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Blog User Del", &email, "hash").await.unwrap();
    let post = insert_test_blog_post(&pool, "Delete Me Blog", user.id).await;

    let deleted = blog_post_repository::soft_delete(&pool, post.id).await.unwrap();
    assert!(deleted);

    let found = blog_post_repository::find_by_slug(&pool, &post.slug).await.unwrap();
    assert!(found.is_none());

    let restored = blog_post_repository::restore(&pool, post.id).await.unwrap();
    assert!(restored);

    let found_restored = blog_post_repository::find_by_slug(&pool, &post.slug).await.unwrap();
    assert!(found_restored.is_some());
}

#[sqlx::test]
async fn test_permanent_delete(pool: PgPool) {
    let email = format!("blog_perm_{}@example.com", Uuid::new_v4());
    let user = user_repository::create(&pool, "Blog User Perm", &email, "hash").await.unwrap();
    let post = insert_test_blog_post(&pool, "Perm Delete Blog", user.id).await;

    blog_post_repository::soft_delete(&pool, post.id).await.unwrap();
    let deleted = blog_post_repository::permanent_delete(&pool, post.id).await.unwrap();
    assert!(deleted);

    // Should not be found at all
    // Since there's no find_by_id_with_deleted for blog posts yet, we can't test fetching it,
    // but we can verify it doesn't show up in normal fetch and that rows_affected was > 0
    let found = blog_post_repository::find_by_slug(&pool, &post.slug).await.unwrap();
    assert!(found.is_none());
}
