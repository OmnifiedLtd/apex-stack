use sqlx::{Executor, FromRow, Postgres};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::DomainError;

/// User entity
#[derive(Debug, Clone, FromRow, PartialEq)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Repository for User operations
pub struct UserRepository;

impl UserRepository {
    /// Create a new user within a transaction
    pub async fn create<'e, E>(
        executor: E,
        email: &str,
        name: &str,
    ) -> Result<User, DomainError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();

        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (id, email, name, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, email, name, created_at, updated_at
            "#,
            id,
            email,
            name,
            now,
            now
        )
        .fetch_one(executor)
        .await?;

        Ok(user)
    }

    /// Find a user by ID
    pub async fn find_by_id<'e, E>(executor: E, id: Uuid) -> Result<Option<User>, DomainError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, email, name, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(executor)
        .await?;

        Ok(user)
    }

    /// Find a user by email
    pub async fn find_by_email<'e, E>(
        executor: E,
        email: &str,
    ) -> Result<Option<User>, DomainError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, email, name, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(executor)
        .await?;

        Ok(user)
    }

    /// List all users
    pub async fn list<'e, E>(executor: E) -> Result<Vec<User>, DomainError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let users = sqlx::query_as!(
            User,
            r#"
            SELECT id, email, name, created_at, updated_at
            FROM users
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(executor)
        .await?;

        Ok(users)
    }

    /// Update a user's name
    pub async fn update_name<'e, E>(
        executor: E,
        id: Uuid,
        name: &str,
    ) -> Result<Option<User>, DomainError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let now = OffsetDateTime::now_utc();

        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET name = $1, updated_at = $2
            WHERE id = $3
            RETURNING id, email, name, created_at, updated_at
            "#,
            name,
            now,
            id
        )
        .fetch_optional(executor)
        .await?;

        Ok(user)
    }

    /// Delete a user by ID
    pub async fn delete<'e, E>(executor: E, id: Uuid) -> Result<bool, DomainError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let result = sqlx::query!(
            r#"
            DELETE FROM users
            WHERE id = $1
            "#,
            id
        )
        .execute(executor)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
