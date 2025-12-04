use domain::{DomainError, UserRepository};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "../../migrations")]
async fn test_create_user(pool: PgPool) -> Result<(), DomainError> {
    let mut tx = pool.begin().await?;

    let user = UserRepository::create(&mut *tx, "test@example.com", "Test User").await?;

    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.name, "Test User");
    assert!(user.created_at <= user.updated_at);

    tx.rollback().await?;
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_create_user_duplicate_email_fails(pool: PgPool) -> Result<(), DomainError> {
    let mut tx = pool.begin().await?;
    
    // Create first user
    UserRepository::create(&mut *tx, "duplicate@example.com", "First").await?;
    
    // Try to create second user with same email in same transaction (should fail)
    // Note: This poisons the transaction, which is fine as we end the test.
    let result = UserRepository::create(&mut *tx, "duplicate@example.com", "Second").await;

    assert!(result.is_err());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_id(pool: PgPool) -> Result<(), DomainError> {
    let mut tx = pool.begin().await?;
    
    // Create a user
    let created = UserRepository::create(&mut *tx, "find@example.com", "Find Me").await?;

    // Find by ID using SAME transaction
    let found = UserRepository::find_by_id(&mut *tx, created.id).await?;

    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, created.id);
    assert_eq!(found.email, "find@example.com");
    
    tx.rollback().await?;
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_id_not_found(pool: PgPool) -> Result<(), DomainError> {
    let mut tx = pool.begin().await?;
    let found = UserRepository::find_by_id(&mut *tx, Uuid::new_v4()).await?;
    assert!(found.is_none());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_email(pool: PgPool) -> Result<(), DomainError> {
    let mut tx = pool.begin().await?;
    
    // Create a user
    let created = UserRepository::create(&mut *tx, "email@example.com", "Email User").await?;

    // Find by email using SAME transaction
    let found = UserRepository::find_by_email(&mut *tx, "email@example.com").await?;

    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, created.id);
    assert_eq!(found.name, "Email User");
    
    tx.rollback().await?;
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_email_not_found(pool: PgPool) -> Result<(), DomainError> {
    let mut tx = pool.begin().await?;
    let found = UserRepository::find_by_email(&mut *tx, "nonexistent@example.com").await?;
    assert!(found.is_none());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_list_users(pool: PgPool) -> Result<(), DomainError> {
    let mut tx = pool.begin().await?;
    
    // Create multiple users in same transaction
    UserRepository::create(&mut *tx, "user1@example.com", "User 1").await?;
    UserRepository::create(&mut *tx, "user2@example.com", "User 2").await?;
    UserRepository::create(&mut *tx, "user3@example.com", "User 3").await?;

    // List all using SAME transaction
    let users = UserRepository::list(&mut *tx).await?;

    assert_eq!(users.len(), 3);
    // Should be ordered by created_at DESC (most recent first)
    assert_eq!(users[0].name, "User 3");
    assert_eq!(users[1].name, "User 2");
    assert_eq!(users[2].name, "User 1");
    
    tx.rollback().await?;
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_list_users_empty(pool: PgPool) -> Result<(), DomainError> {
    let mut tx = pool.begin().await?;
    let users = UserRepository::list(&mut *tx).await?;
    assert!(users.is_empty());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_name(pool: PgPool) -> Result<(), DomainError> {
    let mut tx = pool.begin().await?;
    
    // Create a user
    let created = UserRepository::create(&mut *tx, "update@example.com", "Original Name").await?;

    // Update the name using SAME transaction
    let updated = UserRepository::update_name(&mut *tx, created.id, "New Name").await?;

    assert!(updated.is_some());
    let updated = updated.unwrap();
    assert_eq!(updated.id, created.id);
    assert_eq!(updated.name, "New Name");
    assert_eq!(updated.email, "update@example.com"); // Email unchanged
    assert!(updated.updated_at > created.updated_at);
    
    tx.rollback().await?;
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_name_not_found(pool: PgPool) -> Result<(), DomainError> {
    let mut tx = pool.begin().await?;
    let updated = UserRepository::update_name(&mut *tx, Uuid::new_v4(), "New Name").await?;
    assert!(updated.is_none());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_delete_user(pool: PgPool) -> Result<(), DomainError> {
    let mut tx = pool.begin().await?;
    
    // Create a user
    let created = UserRepository::create(&mut *tx, "delete@example.com", "Delete Me").await?;

    // Delete the user using SAME transaction
    let deleted = UserRepository::delete(&mut *tx, created.id).await?;
    assert!(deleted);

    // Verify it's gone using SAME transaction
    let found = UserRepository::find_by_id(&mut *tx, created.id).await?;
    assert!(found.is_none());
    
    tx.rollback().await?;
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_delete_user_not_found(pool: PgPool) -> Result<(), DomainError> {
    let mut tx = pool.begin().await?;
    let deleted = UserRepository::delete(&mut *tx, Uuid::new_v4()).await?;
    assert!(!deleted);
    Ok(())
}