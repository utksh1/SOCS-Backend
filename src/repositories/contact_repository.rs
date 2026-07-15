use sqlx::PgPool;


use crate::models::contact::Contact;

pub async fn create(
    pool: &PgPool,
    name: &str,
    email: &str,
    subject: &str,
    message: &str,
) -> Result<Contact, sqlx::Error> {
    sqlx::query_as::<_, Contact>(
        r#"
        INSERT INTO contacts (name, email, subject, message)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, email, subject, message, replied, created_at
        "#,
    )
    .bind(name)
    .bind(email)
    .bind(subject)
    .bind(message)
    .fetch_one(pool)
    .await
}

pub async fn find_all(pool: &PgPool) -> Result<Vec<Contact>, sqlx::Error> {
    sqlx::query_as::<_, Contact>(
        r#"
        SELECT id, name, email, subject, message, replied, created_at
        FROM contacts
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

