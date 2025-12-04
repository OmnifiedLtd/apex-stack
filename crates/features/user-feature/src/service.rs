use domain::{User, UserRepository};
use sqlx::{Executor, PgPool, Postgres};
use uuid::Uuid;

use crate::error::UserFeatureError;
use crate::jobs::UserJobs;

/// Input for creating a new user
pub struct CreateUserInput {
    pub email: String,
    pub name: String,
}

/// Input for updating a user
pub struct UpdateUserInput {
    pub name: Option<String>,
}

/// Service for user-related operations
pub struct UserService;

impl UserService {
    /// Register a new user and enqueue a welcome email atomically
    /// 
    /// Requires a Pool to manage the transaction internally.
    pub async fn register(pool: &PgPool, input: CreateUserInput) -> Result<User, UserFeatureError> {
        // Check if email already exists
        // Note: We use the pool here, effectively a separate read. 
        // In high concurrency, a race condition exists here, but the DB constraint will catch it.
        if UserRepository::find_by_email(pool, &input.email)
            .await?
            .is_some()
        {
            return Err(UserFeatureError::EmailExists(input.email));
        }

        // Start transaction for atomic user creation + job enqueue
        let mut tx = pool.begin().await.map_err(domain::DomainError::from)?;

        // Create the user
        let user = UserRepository::create(&mut *tx, &input.email, &input.name).await?;

        // Enqueue the welcome email job within the same transaction
        UserJobs::enqueue_welcome_email(
            &mut tx,
            user.id,
            user.email.clone(),
            user.name.clone(),
        )
        .await
        .map_err(|e| UserFeatureError::Queue(e.to_string()))?;

        // Commit both the user and the job together
        tx.commit().await.map_err(domain::DomainError::from)?;

        Ok(user)
    }

    /// Get a user by ID
    pub async fn get<'e, E>(executor: E, id: Uuid) -> Result<User, UserFeatureError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        UserRepository::find_by_id(executor, id)
            .await?
            .ok_or(UserFeatureError::NotFound(id))
    }

    /// Get a user by email
    pub async fn get_by_email<'e, E>(
        executor: E,
        email: &str,
    ) -> Result<Option<User>, UserFeatureError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        Ok(UserRepository::find_by_email(executor, email).await?)
    }

    /// List all users
    pub async fn list<'e, E>(executor: E) -> Result<Vec<User>, UserFeatureError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        Ok(UserRepository::list(executor).await?)
    }

    /// Update a user
    pub async fn update<'e, E>(
        executor: E,
        id: Uuid,
        input: UpdateUserInput,
    ) -> Result<User, UserFeatureError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        if let Some(name) = input.name {
            UserRepository::update_name(executor, id, &name)
                .await?
                .ok_or(UserFeatureError::NotFound(id))
        } else {
            Self::get(executor, id).await
        }
    }

    /// Delete a user
    pub async fn delete<'e, E>(executor: E, id: Uuid) -> Result<bool, UserFeatureError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        Ok(UserRepository::delete(executor, id).await?)
    }
}
