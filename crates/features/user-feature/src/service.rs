use domain::{User, UserRepository};
use sqlx::PgPool;
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
    pub async fn register(pool: &PgPool, input: CreateUserInput) -> Result<User, UserFeatureError> {
        // Check if email already exists
        if UserRepository::find_by_email(pool, &input.email)
            .await?
            .is_some()
        {
            return Err(UserFeatureError::EmailExists(input.email));
        }

        // Start transaction for atomic user creation + job enqueue
        let mut tx = pool.begin().await.map_err(domain::DomainError::from)?;

        // Create the user
        let user = UserRepository::create(&mut tx, &input.email, &input.name).await?;

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
    pub async fn get(pool: &PgPool, id: Uuid) -> Result<User, UserFeatureError> {
        UserRepository::find_by_id(pool, id)
            .await?
            .ok_or(UserFeatureError::NotFound(id))
    }

    /// Get a user by email
    pub async fn get_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, UserFeatureError> {
        Ok(UserRepository::find_by_email(pool, email).await?)
    }

    /// List all users
    pub async fn list(pool: &PgPool) -> Result<Vec<User>, UserFeatureError> {
        Ok(UserRepository::list(pool).await?)
    }

    /// Update a user
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        input: UpdateUserInput,
    ) -> Result<User, UserFeatureError> {
        if let Some(name) = input.name {
            UserRepository::update_name(pool, id, &name)
                .await?
                .ok_or(UserFeatureError::NotFound(id))
        } else {
            Self::get(pool, id).await
        }
    }

    /// Delete a user
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, UserFeatureError> {
        Ok(UserRepository::delete(pool, id).await?)
    }
}
