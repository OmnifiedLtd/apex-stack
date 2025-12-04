use sea_query::{Expr, Iden, PostgresQueryBuilder, Query};
use sea_query_binder::SqlxBinder;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::DomainError;

/// Schema definition for the users table
#[derive(Iden)]
pub enum Users {
    Table,
    Id,
    Email,
    Name,
    CreatedAt,
    UpdatedAt,
}

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
    pub async fn create(
        tx: &mut Transaction<'_, Postgres>,
        email: &str,
        name: &str,
    ) -> Result<User, DomainError> {
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();

        let (sql, values) = Query::insert()
            .into_table(Users::Table)
            .columns([
                Users::Id,
                Users::Email,
                Users::Name,
                Users::CreatedAt,
                Users::UpdatedAt,
            ])
            .values_panic([
                id.into(),
                email.into(),
                name.into(),
                now.into(),
                now.into(),
            ])
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        let user = sqlx::query_as_with::<_, User, _>(&sql, values)
            .fetch_one(&mut **tx)
            .await?;

        Ok(user)
    }

    /// Find a user by ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, DomainError> {
        let (sql, values) = Query::select()
            .columns([
                Users::Id,
                Users::Email,
                Users::Name,
                Users::CreatedAt,
                Users::UpdatedAt,
            ])
            .from(Users::Table)
            .and_where(Expr::col(Users::Id).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let user = sqlx::query_as_with::<_, User, _>(&sql, values)
            .fetch_optional(pool)
            .await?;

        Ok(user)
    }

    /// Find a user by email
    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, DomainError> {
        let (sql, values) = Query::select()
            .columns([
                Users::Id,
                Users::Email,
                Users::Name,
                Users::CreatedAt,
                Users::UpdatedAt,
            ])
            .from(Users::Table)
            .and_where(Expr::col(Users::Email).eq(email))
            .build_sqlx(PostgresQueryBuilder);

        let user = sqlx::query_as_with::<_, User, _>(&sql, values)
            .fetch_optional(pool)
            .await?;

        Ok(user)
    }

    /// List all users
    pub async fn list(pool: &PgPool) -> Result<Vec<User>, DomainError> {
        let (sql, values) = Query::select()
            .columns([
                Users::Id,
                Users::Email,
                Users::Name,
                Users::CreatedAt,
                Users::UpdatedAt,
            ])
            .from(Users::Table)
            .order_by(Users::CreatedAt, sea_query::Order::Desc)
            .build_sqlx(PostgresQueryBuilder);

        let users = sqlx::query_as_with::<_, User, _>(&sql, values)
            .fetch_all(pool)
            .await?;

        Ok(users)
    }

    /// Update a user's name
    pub async fn update_name(
        pool: &PgPool,
        id: Uuid,
        name: &str,
    ) -> Result<Option<User>, DomainError> {
        let now = OffsetDateTime::now_utc();

        let (sql, values) = Query::update()
            .table(Users::Table)
            .values([
                (Users::Name, name.into()),
                (Users::UpdatedAt, now.into()),
            ])
            .and_where(Expr::col(Users::Id).eq(id))
            .returning_all()
            .build_sqlx(PostgresQueryBuilder);

        let user = sqlx::query_as_with::<_, User, _>(&sql, values)
            .fetch_optional(pool)
            .await?;

        Ok(user)
    }

    /// Delete a user by ID
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, DomainError> {
        let (sql, values) = Query::delete()
            .from_table(Users::Table)
            .and_where(Expr::col(Users::Id).eq(id))
            .build_sqlx(PostgresQueryBuilder);

        let result = sqlx::query_with(&sql, values).execute(pool).await?;

        Ok(result.rows_affected() > 0)
    }
}
