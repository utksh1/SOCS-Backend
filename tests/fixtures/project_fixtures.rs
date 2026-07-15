use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use socs_backend::models::project::{Project, ContentStatus};

pub struct ProjectFixture {
    pub title: String,
    pub description: String,
    pub tech_stack: Vec<String>,
    pub created_by: Uuid,
    pub status: ContentStatus,
    pub github_link: Option<String>,
    pub tags: Vec<String>,
    pub featured: bool,
}

impl ProjectFixture {
    pub fn new(created_by: Uuid) -> Self {
        let id = Uuid::new_v4();
        Self {
            title: format!("Test Project {}", &id.to_string()[..8]),
            description: "Test project description".to_string(),
            tech_stack: vec!["Rust".to_string(), "PostgreSQL".to_string()],
            created_by,
            status: ContentStatus::Pending,
            github_link: None,
            tags: vec![],
            featured: false,
        }
    }
    
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
    
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
    
    pub fn tech_stack(mut self, tech_stack: Vec<String>) -> Self {
        self.tech_stack = tech_stack;
        self
    }
    
    pub fn status(mut self, status: ContentStatus) -> Self {
        self.status = status;
        self
    }
    
    pub fn approved(mut self) -> Self {
        self.status = ContentStatus::Approved;
        self
    }
    
    pub fn github_link(mut self, url: impl Into<String>) -> Self {
        self.github_link = Some(url.into());
        self
    }
    
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
    
    pub fn featured(mut self) -> Self {
        self.featured = true;
        self
    }
    
    /// Insert project into database
    pub async fn insert(self, pool: &PgPool) -> Project {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let slug = self.title.to_lowercase().replace(" ", "-");
        
        let project = sqlx::query_as!(
            Project,
            r#"
            INSERT INTO projects (
                id, slug, title, description, tech_stack, github_link,
                tags, featured, status, created_by, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING 
                id, slug, title, description, tech_stack, github_link,
                tags, featured,
                status as "status: ContentStatus",
                approved_by, approved_at, created_by,
                created_at, updated_at, deleted_at
            "#,
            id,
            slug,
            self.title,
            self.description,
            &self.tech_stack,
            self.github_link,
            &self.tags,
            self.featured,
            self.status as ContentStatus,
            Some(self.created_by),
            now,
            now
        )
        .fetch_one(pool)
        .await
        .unwrap();
        
        project
    }
    
    /// Add collaborators to project
    pub async fn add_collaborators(self, pool: &PgPool, collaborator_ids: Vec<Uuid>) -> Project {
        let project = self.insert(pool).await;
        
        for collaborator_id in collaborator_ids {
            sqlx::query!(
                r#"
                INSERT INTO project_collaborators (id, project_id, user_id, created_at)
                VALUES ($1, $2, $3, $4)
                "#,
                Uuid::new_v4(),
                project.id,
                collaborator_id,
                Utc::now()
            )
            .execute(pool)
            .await
            .unwrap();
        }
        
        project
    }
}
