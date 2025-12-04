use domain::{DomainError, UserRepository};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "../../migrations")]
async fn test_create_user(pool: PgPool) -> Result<(), DomainError> {
    let mut tx = pool.begin().await?;

    let user = UserRepository::create(&mut tx, "test@example.com", "Test User").await?;

    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.name, "Test User");
    assert!(user.created_at <= user.updated_at);

    tx.commit().await?;
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_create_user_duplicate_email_fails(pool: PgPool) -> Result<(), DomainError> {
    // Create first user
    let mut tx = pool.begin().await?;
    UserRepository::create(&mut tx, "duplicate@example.com", "First").await?;
    tx.commit().await?;

    // Try to create second user with same email
    let mut tx = pool.begin().await?;
    let result = UserRepository::create(&mut tx, "duplicate@example.com", "Second").await;

    assert!(result.is_err());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_id(pool: PgPool) -> Result<(), DomainError> {
    // Create a user
    let mut tx = pool.begin().await?;
    let created = UserRepository::create(&mut tx, "find@example.com", "Find Me").await?;
    tx.commit().await?;

    // Find by ID
    let found = UserRepository::find_by_id(&pool, created.id).await?;

    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, created.id);
    assert_eq!(found.email, "find@example.com");
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_id_not_found(pool: PgPool) -> Result<(), DomainError> {
    let found = UserRepository::find_by_id(&pool, Uuid::new_v4()).await?;
    assert!(found.is_none());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_email(pool: PgPool) -> Result<(), DomainError> {
    // Create a user
    let mut tx = pool.begin().await?;
    let created = UserRepository::create(&mut tx, "email@example.com", "Email User").await?;
    tx.commit().await?;

    // Find by email
    let found = UserRepository::find_by_email(&pool, "email@example.com").await?;

    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, created.id);
    assert_eq!(found.name, "Email User");
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_email_not_found(pool: PgPool) -> Result<(), DomainError> {
    let found = UserRepository::find_by_email(&pool, "nonexistent@example.com").await?;
    assert!(found.is_none());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_list_users(pool: PgPool) -> Result<(), DomainError> {
    // Create multiple users
    let mut tx = pool.begin().await?;
    UserRepository::create(&mut tx, "user1@example.com", "User 1").await?;
    tx.commit().await?;

    let mut tx = pool.begin().await?;
    UserRepository::create(&mut tx, "user2@example.com", "User 2").await?;
    tx.commit().await?;

    let mut tx = pool.begin().await?;
    UserRepository::create(&mut tx, "user3@example.com", "User 3").await?;
    tx.commit().await?;

    // List all
    let users = UserRepository::list(&pool).await?;

    assert_eq!(users.len(), 3);
    // Should be ordered by created_at DESC (most recent first)
    assert_eq!(users[0].name, "User 3");
    assert_eq!(users[1].name, "User 2");
    assert_eq!(users[2].name, "User 1");
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_list_users_empty(pool: PgPool) -> Result<(), DomainError> {
    let users = UserRepository::list(&pool).await?;
    assert!(users.is_empty());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_name(pool: PgPool) -> Result<(), DomainError> {
    // Create a user
    let mut tx = pool.begin().await?;
    let created = UserRepository::create(&mut tx, "update@example.com", "Original Name").await?;
    tx.commit().await?;

    // Update the name
    let updated = UserRepository::update_name(&pool, created.id, "New Name").await?;

    assert!(updated.is_some());
    let updated = updated.unwrap();
    assert_eq!(updated.id, created.id);
    assert_eq!(updated.name, "New Name");
    assert_eq!(updated.email, "update@example.com"); // Email unchanged
    assert!(updated.updated_at > created.updated_at);
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_name_not_found(pool: PgPool) -> Result<(), DomainError> {
    let updated = UserRepository::update_name(&pool, Uuid::new_v4(), "New Name").await?;
    assert!(updated.is_none());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_delete_user(pool: PgPool) -> Result<(), DomainError> {
    // Create a user
    let mut tx = pool.begin().await?;
    let created = UserRepository::create(&mut tx, "delete@example.com", "Delete Me").await?;
    tx.commit().await?;

    // Delete the user
    let deleted = UserRepository::delete(&pool, created.id).await?;
    assert!(deleted);

    // Verify it's gone
    let found = UserRepository::find_by_id(&pool, created.id).await?;
    assert!(found.is_none());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_delete_user_not_found(pool: PgPool) -> Result<(), DomainError> {
    let deleted = UserRepository::delete(&pool, Uuid::new_v4()).await?;
    assert!(!deleted);
    Ok(())
}
