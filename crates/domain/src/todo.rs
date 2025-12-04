use sqlx::{Executor, FromRow, Postgres};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::DomainError;

/// Todo status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

impl TodoStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TodoStatus::Pending => "pending",
            TodoStatus::InProgress => "in_progress",
            TodoStatus::Completed => "completed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(TodoStatus::Pending),
            "in_progress" => Some(TodoStatus::InProgress),
            "completed" => Some(TodoStatus::Completed),
            _ => None,
        }
    }
}

/// Raw todo row from database
#[derive(Debug, Clone, FromRow)]
struct TodoRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Todo entity
#[derive(Debug, Clone, PartialEq)]
pub struct Todo {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TodoStatus,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<TodoRow> for Todo {
    fn from(row: TodoRow) -> Self {
        Todo {
            id: row.id,
            user_id: row.user_id,
            title: row.title,
            description: row.description,
            status: TodoStatus::from_str(&row.status).unwrap_or(TodoStatus::Pending),
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Repository for Todo operations
pub struct TodoRepository;

impl TodoRepository {
    /// Create a new todo
    pub async fn create<'e, E>(
        executor: E,
        user_id: Uuid,
        title: &str,
        description: Option<&str>,
    ) -> Result<Todo, DomainError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();
        let status = TodoStatus::Pending.as_str();
        let description_string = description.map(|s| s.to_string());

        let row = sqlx::query_as!(
            TodoRow,
            r#"
            INSERT INTO todos (id, user_id, title, description, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, user_id, title, description, status, created_at, updated_at
            "#,
            id,
            user_id,
            title,
            description_string,
            status,
            now,
            now
        )
        .fetch_one(executor)
        .await?;

        Ok(row.into())
    }

    /// Find a todo by ID
    pub async fn find_by_id<'e, E>(executor: E, id: Uuid) -> Result<Option<Todo>, DomainError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let row = sqlx::query_as!(
            TodoRow,
            r#"
            SELECT id, user_id, title, description, status, created_at, updated_at
            FROM todos
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(executor)
        .await?;

        Ok(row.map(Into::into))
    }

    /// List todos for a user
    pub async fn list_by_user<'e, E>(executor: E, user_id: Uuid) -> Result<Vec<Todo>, DomainError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let rows = sqlx::query_as!(
            TodoRow,
            r#"
            SELECT id, user_id, title, description, status, created_at, updated_at
            FROM todos
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(executor)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// List todos by status for a user
    pub async fn list_by_user_and_status<'e, E>(
        executor: E,
        user_id: Uuid,
        status: TodoStatus,
    ) -> Result<Vec<Todo>, DomainError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let status_str = status.as_str();
        let rows = sqlx::query_as!(
            TodoRow,
            r#"
            SELECT id, user_id, title, description, status, created_at, updated_at
            FROM todos
            WHERE user_id = $1 AND status = $2
            ORDER BY created_at DESC
            "#,
            user_id,
            status_str
        )
        .fetch_all(executor)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Update a todo's status
    pub async fn update_status<'e, E>(
        executor: E,
        id: Uuid,
        status: TodoStatus,
    ) -> Result<Option<Todo>, DomainError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let now = OffsetDateTime::now_utc();
        let status_str = status.as_str();

        let row = sqlx::query_as!(
            TodoRow,
            r#"
            UPDATE todos
            SET status = $1, updated_at = $2
            WHERE id = $3
            RETURNING id, user_id, title, description, status, created_at, updated_at
            "#,
            status_str,
            now,
            id
        )
        .fetch_optional(executor)
        .await?;

        Ok(row.map(Into::into))
    }

    /// Update a todo's title and description
    pub async fn update_content<'e, E>(
        executor: E,
        id: Uuid,
        title: &str,
        description: Option<&str>,
    ) -> Result<Option<Todo>, DomainError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let now = OffsetDateTime::now_utc();
        let description_string = description.map(|s| s.to_string());

        let row = sqlx::query_as!(
            TodoRow,
            r#"
            UPDATE todos
            SET title = $1, description = $2, updated_at = $3
            WHERE id = $4
            RETURNING id, user_id, title, description, status, created_at, updated_at
            "#,
            title,
            description_string,
            now,
            id
        )
        .fetch_optional(executor)
        .await?;

        Ok(row.map(Into::into))
    }

    /// Delete a todo
    pub async fn delete<'e, E>(executor: E, id: Uuid) -> Result<bool, DomainError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query!(
            r#"
            DELETE FROM todos
            WHERE id = $1
            "#,
            id
        )
        .execute(executor)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}