use sea_query::{Expr, Iden, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::DomainError;

/// Schema definition for the todos table
#[derive(Iden)]
pub enum Todos {
    Table,
    Id,
    UserId,
    Title,
    Description,
    Status,
    CreatedAt,
    UpdatedAt,
}

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

impl From<TodoStatus> for sea_query::Value {
    fn from(status: TodoStatus) -> Self {
        status.as_str().into()
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
    pub async fn create(
        pool: &PgPool,
        user_id: Uuid,
        title: &str,
        description: Option<&str>,
    ) -> Result<Todo, DomainError> {
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();

        let (sql, values) = Query::insert()
            .into_table(Todos::Table)
            .columns([
                Todos::Id,
                Todos::UserId,
                Todos::Title,
                Todos::Description,
                Todos::Status,
                Todos::CreatedAt,
                Todos::UpdatedAt,
            ])
            .values_panic([
                id.into(),
                user_id.into(),
                title.into(),
                description.map(|s| s.to_string()).into(),
                TodoStatus::Pending.into(),
                now.into(),
                now.into(),
            ])
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_as_with::<_, TodoRow, _>(&sql, values)
            .fetch_one(pool)
            .await?;

        Ok(row.into())
    }

    /// Find a todo by ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Todo>, DomainError> {
        let (sql, values) = Query::select()
            .columns([
                Todos::Id,
                Todos::UserId,
                Todos::Title,
                Todos::Description,
                Todos::Status,
                Todos::CreatedAt,
                Todos::UpdatedAt,
            ])
            .from(Todos::Table)
            .and_where(Expr::col(Todos::Id).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_as_with::<_, TodoRow, _>(&sql, values)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(Into::into))
    }

    /// List todos for a user
    pub async fn list_by_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<Todo>, DomainError> {
        let (sql, values) = Query::select()
            .columns([
                Todos::Id,
                Todos::UserId,
                Todos::Title,
                Todos::Description,
                Todos::Status,
                Todos::CreatedAt,
                Todos::UpdatedAt,
            ])
            .from(Todos::Table)
            .and_where(Expr::col(Todos::UserId).eq(user_id))
            .order_by(Todos::CreatedAt, sea_query::Order::Desc)
            .build_sqlx(PostgresQueryBuilder);

        let rows = sqlx::query_as_with::<_, TodoRow, _>(&sql, values)
            .fetch_all(pool)
            .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// List todos by status for a user
    pub async fn list_by_user_and_status(
        pool: &PgPool,
        user_id: Uuid,
        status: TodoStatus,
    ) -> Result<Vec<Todo>, DomainError> {
        let (sql, values) = Query::select()
            .columns([
                Todos::Id,
                Todos::UserId,
                Todos::Title,
                Todos::Description,
                Todos::Status,
                Todos::CreatedAt,
                Todos::UpdatedAt,
            ])
            .from(Todos::Table)
            .and_where(Expr::col(Todos::UserId).eq(user_id))
            .and_where(Expr::col(Todos::Status).eq(status))
            .order_by(Todos::CreatedAt, sea_query::Order::Desc)
            .build_sqlx(PostgresQueryBuilder);

        let rows = sqlx::query_as_with::<_, TodoRow, _>(&sql, values)
            .fetch_all(pool)
            .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Update a todo's status
    pub async fn update_status(
        pool: &PgPool,
        id: Uuid,
        status: TodoStatus,
    ) -> Result<Option<Todo>, DomainError> {
        let now = OffsetDateTime::now_utc();

        let (sql, values) = Query::update()
            .table(Todos::Table)
            .values([
                (Todos::Status, status.into()),
                (Todos::UpdatedAt, now.into()),
            ])
            .and_where(Expr::col(Todos::Id).eq(id))
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_as_with::<_, TodoRow, _>(&sql, values)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(Into::into))
    }

    /// Update a todo's title and description
    pub async fn update_content(
        pool: &PgPool,
        id: Uuid,
        title: &str,
        description: Option<&str>,
    ) -> Result<Option<Todo>, DomainError> {
        let now = OffsetDateTime::now_utc();

        let (sql, values) = Query::update()
            .table(Todos::Table)
            .values([
                (Todos::Title, title.into()),
                (Todos::Description, description.map(|s| s.to_string()).into()),
                (Todos::UpdatedAt, now.into()),
            ])
            .and_where(Expr::col(Todos::Id).eq(id))
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        let row = sqlx::query_as_with::<_, TodoRow, _>(&sql, values)
            .fetch_optional(pool)
            .await?;

        Ok(row.map(Into::into))
    }

    /// Delete a todo
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, DomainError> {
        let (sql, values) = Query::delete()
            .from_table(Todos::Table)
            .and_where(Expr::col(Todos::Id).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let result = sqlx::query_with(&sql, values).execute(pool).await?;

        Ok(result.rows_affected() > 0)
    }
}
