//! User journey tests - end-to-end user workflows at the feature layer
//!
//! These tests verify complete user journeys through the system.
//! They are transport-agnostic (no GraphQL, no HTTP).

use sqlx::PgPool;
use user_feature::{CreateUserInput, UpdateUserInput, UserFeatureError, UserService};

// =============================================================================
// User Registration Journey
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn complete_user_registration_journey(pool: PgPool) -> Result<(), UserFeatureError> {
    // A new user registers with the system
    let user = UserService::register(
        &pool,
        CreateUserInput {
            email: "journey@example.com".to_string(),
            name: "Journey User".to_string(),
        },
    )
    .await?;

    // The user exists and can be queried by ID
    let found_by_id = UserService::get(&pool, user.id).await?;
    assert_eq!(found_by_id.email, "journey@example.com");
    assert_eq!(found_by_id.name, "Journey User");

    // The user can also be found by email
    let found_by_email = UserService::get_by_email(&pool, "journey@example.com").await?;
    assert!(found_by_email.is_some());
    assert_eq!(found_by_email.unwrap().id, user.id);

    // A welcome email job was enqueued (transactional atomicity)
    let email_job_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM mq_msgs WHERE channel_name = 'emails' AND id != uuid_nil()",
    )
    .fetch_one(&pool)
    .await
    .map_err(domain::DomainError::from)?;
    assert_eq!(email_job_count, 1);

    // The user appears in the user list
    let users = UserService::list(&pool).await?;
    assert!(users.iter().any(|u| u.id == user.id));

    Ok(())
}

// =============================================================================
// User Profile Update Journey
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn user_can_update_their_profile(pool: PgPool) -> Result<(), UserFeatureError> {
    // Given a registered user
    let original = UserService::register(
        &pool,
        CreateUserInput {
            email: "profile@example.com".to_string(),
            name: "Original Name".to_string(),
        },
    )
    .await?;

    // When the user updates their name
    let updated = UserService::update(
        &pool,
        original.id,
        UpdateUserInput {
            name: Some("New Name".to_string()),
        },
    )
    .await?;

    // Then the name is changed
    assert_eq!(updated.name, "New Name");
    // And the email remains unchanged (email is immutable)
    assert_eq!(updated.email, "profile@example.com");
    // And the ID remains the same
    assert_eq!(updated.id, original.id);

    // The change persists when re-queried
    let refetched = UserService::get(&pool, original.id).await?;
    assert_eq!(refetched.name, "New Name");

    Ok(())
}

// =============================================================================
// User Account Deletion Journey
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn user_account_deletion_journey(pool: PgPool) -> Result<(), UserFeatureError> {
    // Given a registered user
    let user = UserService::register(
        &pool,
        CreateUserInput {
            email: "todelete@example.com".to_string(),
            name: "To Delete".to_string(),
        },
    )
    .await?;

    // Confirm the user exists
    let exists = UserService::get(&pool, user.id).await;
    assert!(exists.is_ok());

    // When the user account is deleted
    let deleted = UserService::delete(&pool, user.id).await?;
    assert!(deleted);

    // Then the user can no longer be found by ID
    let by_id = UserService::get(&pool, user.id).await;
    assert!(matches!(by_id, Err(UserFeatureError::NotFound(_))));

    // And the user can no longer be found by email
    let by_email = UserService::get_by_email(&pool, "todelete@example.com").await?;
    assert!(by_email.is_none());

    // And the user no longer appears in the list
    let users = UserService::list(&pool).await?;
    assert!(!users.iter().any(|u| u.id == user.id));

    Ok(())
}

// =============================================================================
// Multi-User Scenario
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn multiple_users_can_coexist(pool: PgPool) -> Result<(), UserFeatureError> {
    // Register multiple users
    let alice = UserService::register(
        &pool,
        CreateUserInput {
            email: "alice@example.com".to_string(),
            name: "Alice".to_string(),
        },
    )
    .await?;

    let bob = UserService::register(
        &pool,
        CreateUserInput {
            email: "bob@example.com".to_string(),
            name: "Bob".to_string(),
        },
    )
    .await?;

    let charlie = UserService::register(
        &pool,
        CreateUserInput {
            email: "charlie@example.com".to_string(),
            name: "Charlie".to_string(),
        },
    )
    .await?;

    // All users exist independently
    assert_eq!(UserService::get(&pool, alice.id).await?.name, "Alice");
    assert_eq!(UserService::get(&pool, bob.id).await?.name, "Bob");
    assert_eq!(UserService::get(&pool, charlie.id).await?.name, "Charlie");

    // All users appear in the list
    let users = UserService::list(&pool).await?;
    assert_eq!(users.len(), 3);

    // Updating one user doesn't affect others
    UserService::update(
        &pool,
        alice.id,
        UpdateUserInput {
            name: Some("Alice Updated".to_string()),
        },
    )
    .await?;

    assert_eq!(
        UserService::get(&pool, alice.id).await?.name,
        "Alice Updated"
    );
    assert_eq!(UserService::get(&pool, bob.id).await?.name, "Bob");
    assert_eq!(UserService::get(&pool, charlie.id).await?.name, "Charlie");

    // Deleting one user doesn't affect others
    UserService::delete(&pool, bob.id).await?;

    let remaining = UserService::list(&pool).await?;
    assert_eq!(remaining.len(), 2);
    assert!(remaining.iter().any(|u| u.id == alice.id));
    assert!(remaining.iter().any(|u| u.id == charlie.id));
    assert!(!remaining.iter().any(|u| u.id == bob.id));

    Ok(())
}
