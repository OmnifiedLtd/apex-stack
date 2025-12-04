use sqlx::PgPool;
use user_feature::{CreateUserInput, UpdateUserInput, UserFeatureError, UserService};
use uuid::Uuid;

#[sqlx::test(migrations = "../../../migrations")]
async fn test_register_user(pool: PgPool) -> Result<(), UserFeatureError> {
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
async fn test_register_user_enqueues_welcome_email(pool: PgPool) -> Result<(), UserFeatureError> {
    // Register a user
    let user = UserService::register(
        &pool,
        CreateUserInput {
            email: "welcome@example.com".to_string(),
            name: "Welcome Test".to_string(),
        },
    )
    .await?;

    // Verify a job was enqueued in the 'emails' channel
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

    // Verify the job payload contains the user's email
    // Payloads are stored in the mq_payloads table, joined by id
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

    // User should exist
    let found = UserService::get(&pool, user.id).await?;
    assert_eq!(found.email, "welcome@example.com");

    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_register_duplicate_email_fails(pool: PgPool) -> Result<(), UserFeatureError> {
    // Register first user
    UserService::register(
        &pool,
        CreateUserInput {
            email: "duplicate@example.com".to_string(),
            name: "First".to_string(),
        },
    )
    .await?;

    // Try to register second user with same email
    let result = UserService::register(
        &pool,
        CreateUserInput {
            email: "duplicate@example.com".to_string(),
            name: "Second".to_string(),
        },
    )
    .await;

    assert!(matches!(result, Err(UserFeatureError::EmailExists(_))));
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_get_user(pool: PgPool) -> Result<(), UserFeatureError> {
    let created = UserService::register(
        &pool,
        CreateUserInput {
            email: "get@example.com".to_string(),
            name: "Get Test".to_string(),
        },
    )
    .await?;

    let found = UserService::get(&pool, created.id).await?;

    assert_eq!(found.id, created.id);
    assert_eq!(found.email, "get@example.com");
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_get_user_not_found(pool: PgPool) -> Result<(), UserFeatureError> {
    let result = UserService::get(&pool, Uuid::new_v4()).await;

    assert!(matches!(result, Err(UserFeatureError::NotFound(_))));
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_get_by_email(pool: PgPool) -> Result<(), UserFeatureError> {
    UserService::register(
        &pool,
        CreateUserInput {
            email: "byemail@example.com".to_string(),
            name: "Email Test".to_string(),
        },
    )
    .await?;

    let found = UserService::get_by_email(&pool, "byemail@example.com").await?;

    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "Email Test");
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_get_by_email_not_found(pool: PgPool) -> Result<(), UserFeatureError> {
    let found = UserService::get_by_email(&pool, "nonexistent@example.com").await?;
    assert!(found.is_none());
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_list_users(pool: PgPool) -> Result<(), UserFeatureError> {
    // Register multiple users
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

    let users = UserService::list(&pool).await?;

    assert_eq!(users.len(), 2);
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_list_users_empty(pool: PgPool) -> Result<(), UserFeatureError> {
    let users = UserService::list(&pool).await?;
    assert!(users.is_empty());
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_update_user_name(pool: PgPool) -> Result<(), UserFeatureError> {
    let created = UserService::register(
        &pool,
        CreateUserInput {
            email: "update@example.com".to_string(),
            name: "Original".to_string(),
        },
    )
    .await?;

    let updated = UserService::update(
        &pool,
        created.id,
        UpdateUserInput {
            name: Some("Updated".to_string()),
        },
    )
    .await?;

    assert_eq!(updated.name, "Updated");
    assert_eq!(updated.email, "update@example.com"); // Email unchanged
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_update_user_no_changes(pool: PgPool) -> Result<(), UserFeatureError> {
    let created = UserService::register(
        &pool,
        CreateUserInput {
            email: "nochange@example.com".to_string(),
            name: "No Change".to_string(),
        },
    )
    .await?;

    // Update with no fields set should return existing user
    let updated = UserService::update(&pool, created.id, UpdateUserInput { name: None }).await?;

    assert_eq!(updated.name, "No Change");
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_update_user_not_found(pool: PgPool) -> Result<(), UserFeatureError> {
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

#[sqlx::test(migrations = "../../../migrations")]
async fn test_delete_user(pool: PgPool) -> Result<(), UserFeatureError> {
    let created = UserService::register(
        &pool,
        CreateUserInput {
            email: "delete@example.com".to_string(),
            name: "Delete Test".to_string(),
        },
    )
    .await?;

    let deleted = UserService::delete(&pool, created.id).await?;
    assert!(deleted);

    // Verify user is gone
    let result = UserService::get(&pool, created.id).await;
    assert!(matches!(result, Err(UserFeatureError::NotFound(_))));
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_delete_user_not_found(pool: PgPool) -> Result<(), UserFeatureError> {
    let deleted = UserService::delete(&pool, Uuid::new_v4()).await?;
    assert!(!deleted);
    Ok(())
}
