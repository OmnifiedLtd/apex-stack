//! BDD-style behavior tests for the User feature
//!
//! These tests verify user-related business behaviors work correctly.
//! Focus on user journeys and business rules, not implementation details.

use sqlx::PgPool;
use user_feature::{CreateUserInput, UpdateUserInput, UserFeatureError, UserService};
use uuid::Uuid;

// =============================================================================
// User Registration Behaviors
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn user_can_register_with_email_and_name(pool: PgPool) -> Result<(), UserFeatureError> {
    let user = UserService::register(
        &pool,
        CreateUserInput {
            email: "register@example.com".to_string(),
            name: "Register Test".to_string(),
        },
    )
    .await?;

    assert_eq!(user.email, "register@example.com");
    assert_eq!(user.name, "Register Test");
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn registered_user_receives_welcome_email_job(pool: PgPool) -> Result<(), UserFeatureError> {
    // When a user registers
    let user = UserService::register(
        &pool,
        CreateUserInput {
            email: "welcome@example.com".to_string(),
            name: "Welcome Test".to_string(),
        },
    )
    .await?;

    // Then a welcome email job is enqueued
    // Note: mq_msgs has a dummy row with uuid_nil(), so we exclude it
    let email_job_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM mq_msgs WHERE channel_name = 'emails' AND id != uuid_nil()",
    )
    .fetch_one(&pool)
    .await
    .map_err(domain::DomainError::from)?;

    assert_eq!(
        email_job_count, 1,
        "Expected exactly one job in the 'emails' channel"
    );

    // And the job payload contains the user's email and name
    let payload: String = sqlx::query_scalar(
        "SELECT payload_json::TEXT FROM mq_payloads p
         JOIN mq_msgs m ON p.id = m.id
         WHERE m.channel_name = 'emails' LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .map_err(domain::DomainError::from)?;

    assert!(
        payload.contains("welcome@example.com"),
        "Job payload should contain user email"
    );
    assert!(
        payload.contains("Welcome Test"),
        "Job payload should contain user name"
    );

    // And the user exists in the system
    let found = UserService::get(&pool, user.id).await?;
    assert_eq!(found.email, "welcome@example.com");

    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn duplicate_email_registration_is_rejected(pool: PgPool) -> Result<(), UserFeatureError> {
    // Given an existing user
    UserService::register(
        &pool,
        CreateUserInput {
            email: "duplicate@example.com".to_string(),
            name: "First".to_string(),
        },
    )
    .await?;

    // When another user tries to register with the same email
    let result = UserService::register(
        &pool,
        CreateUserInput {
            email: "duplicate@example.com".to_string(),
            name: "Second".to_string(),
        },
    )
    .await;

    // Then the registration is rejected
    assert!(matches!(result, Err(UserFeatureError::EmailExists(_))));
    Ok(())
}

// =============================================================================
// User Query Behaviors
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn user_can_be_found_by_id(pool: PgPool) -> Result<(), UserFeatureError> {
    // Given a registered user
    let created = UserService::register(
        &pool,
        CreateUserInput {
            email: "get@example.com".to_string(),
            name: "Get Test".to_string(),
        },
    )
    .await?;

    // When querying by ID
    let found = UserService::get(&pool, created.id).await?;

    // Then the user is found with correct data
    assert_eq!(found.id, created.id);
    assert_eq!(found.email, "get@example.com");
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn querying_nonexistent_user_returns_not_found(pool: PgPool) -> Result<(), UserFeatureError> {
    let result = UserService::get(&pool, Uuid::new_v4()).await;

    assert!(matches!(result, Err(UserFeatureError::NotFound(_))));
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn user_can_be_found_by_email(pool: PgPool) -> Result<(), UserFeatureError> {
    // Given a registered user
    UserService::register(
        &pool,
        CreateUserInput {
            email: "byemail@example.com".to_string(),
            name: "Email Test".to_string(),
        },
    )
    .await?;

    // When querying by email
    let found = UserService::get_by_email(&pool, "byemail@example.com").await?;

    // Then the user is found
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Email Test");
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn querying_nonexistent_email_returns_none(pool: PgPool) -> Result<(), UserFeatureError> {
    let found = UserService::get_by_email(&pool, "nonexistent@example.com").await?;
    assert!(found.is_none());
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn all_users_can_be_listed(pool: PgPool) -> Result<(), UserFeatureError> {
    // Given multiple registered users
    UserService::register(
        &pool,
        CreateUserInput {
            email: "list1@example.com".to_string(),
            name: "List 1".to_string(),
        },
    )
    .await?;

    UserService::register(
        &pool,
        CreateUserInput {
            email: "list2@example.com".to_string(),
            name: "List 2".to_string(),
        },
    )
    .await?;

    // When listing all users
    let users = UserService::list(&pool).await?;

    // Then all users are returned
    assert_eq!(users.len(), 2);
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn listing_users_when_none_exist_returns_empty(pool: PgPool) -> Result<(), UserFeatureError> {
    let users = UserService::list(&pool).await?;
    assert!(users.is_empty());
    Ok(())
}

// =============================================================================
// User Update Behaviors
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn user_name_can_be_updated(pool: PgPool) -> Result<(), UserFeatureError> {
    // Given a registered user
    let created = UserService::register(
        &pool,
        CreateUserInput {
            email: "update@example.com".to_string(),
            name: "Original".to_string(),
        },
    )
    .await?;

    // When updating the name
    let updated = UserService::update(
        &pool,
        created.id,
        UpdateUserInput {
            name: Some("Updated".to_string()),
        },
    )
    .await?;

    // Then the name is changed
    assert_eq!(updated.name, "Updated");
    // And the email remains unchanged
    assert_eq!(updated.email, "update@example.com");
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn update_with_no_changes_preserves_user(pool: PgPool) -> Result<(), UserFeatureError> {
    // Given a registered user
    let created = UserService::register(
        &pool,
        CreateUserInput {
            email: "nochange@example.com".to_string(),
            name: "No Change".to_string(),
        },
    )
    .await?;

    // When updating with no fields set
    let updated = UserService::update(&pool, created.id, UpdateUserInput { name: None }).await?;

    // Then the user is unchanged
    assert_eq!(updated.name, "No Change");
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn updating_nonexistent_user_fails(pool: PgPool) -> Result<(), UserFeatureError> {
    let result = UserService::update(
        &pool,
        Uuid::new_v4(),
        UpdateUserInput {
            name: Some("New Name".to_string()),
        },
    )
    .await;

    assert!(matches!(result, Err(UserFeatureError::NotFound(_))));
    Ok(())
}

// =============================================================================
// User Deletion Behaviors
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn user_can_be_deleted(pool: PgPool) -> Result<(), UserFeatureError> {
    // Given a registered user
    let created = UserService::register(
        &pool,
        CreateUserInput {
            email: "delete@example.com".to_string(),
            name: "Delete Test".to_string(),
        },
    )
    .await?;

    // When deleting the user
    let deleted = UserService::delete(&pool, created.id).await?;
    assert!(deleted);

    // Then the user no longer exists
    let result = UserService::get(&pool, created.id).await;
    assert!(matches!(result, Err(UserFeatureError::NotFound(_))));
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn deleting_nonexistent_user_returns_false(pool: PgPool) -> Result<(), UserFeatureError> {
    let deleted = UserService::delete(&pool, Uuid::new_v4()).await?;
    assert!(!deleted);
    Ok(())
}
