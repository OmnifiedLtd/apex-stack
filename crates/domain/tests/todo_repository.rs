use domain::{DomainError, TodoRepository, TodoStatus, UserRepository};
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to create a user for todo tests (todos require a valid user_id)
async fn create_test_user(pool: &PgPool, email: &str) -> Result<Uuid, DomainError> {
    let mut tx = pool.begin().await?;
    let user = UserRepository::create(&mut tx, email, "Test User").await?;
    tx.commit().await?;
    Ok(user.id)
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_create_todo(pool: PgPool) -> Result<(), DomainError> {
    let user_id = create_test_user(&pool, "todo@example.com").await?;

    let todo = TodoRepository::create(&pool, user_id, "My Task", Some("A description")).await?;

    assert_eq!(todo.user_id, user_id);
    assert_eq!(todo.title, "My Task");
    assert_eq!(todo.description, Some("A description".to_string()));
    assert_eq!(todo.status, TodoStatus::Pending);
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_create_todo_without_description(pool: PgPool) -> Result<(), DomainError> {
    let user_id = create_test_user(&pool, "todo2@example.com").await?;

    let todo = TodoRepository::create(&pool, user_id, "Simple Task", None).await?;

    assert_eq!(todo.title, "Simple Task");
    assert!(todo.description.is_none());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_create_todo_invalid_user_fails(pool: PgPool) -> Result<(), DomainError> {
    let fake_user_id = Uuid::new_v4();

    let result = TodoRepository::create(&pool, fake_user_id, "Task", None).await;

    assert!(result.is_err());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_id(pool: PgPool) -> Result<(), DomainError> {
    let user_id = create_test_user(&pool, "find-todo@example.com").await?;
    let created = TodoRepository::create(&pool, user_id, "Find Me", None).await?;

    let found = TodoRepository::find_by_id(&pool, created.id).await?;

    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, created.id);
    assert_eq!(found.title, "Find Me");
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_find_by_id_not_found(pool: PgPool) -> Result<(), DomainError> {
    let found = TodoRepository::find_by_id(&pool, Uuid::new_v4()).await?;
    assert!(found.is_none());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_list_by_user(pool: PgPool) -> Result<(), DomainError> {
    let user_id = create_test_user(&pool, "list-todos@example.com").await?;

    // Create multiple todos
    TodoRepository::create(&pool, user_id, "Task 1", None).await?;
    TodoRepository::create(&pool, user_id, "Task 2", None).await?;
    TodoRepository::create(&pool, user_id, "Task 3", None).await?;

    let todos = TodoRepository::list_by_user(&pool, user_id).await?;

    assert_eq!(todos.len(), 3);
    // Should be ordered by created_at DESC
    assert_eq!(todos[0].title, "Task 3");
    assert_eq!(todos[1].title, "Task 2");
    assert_eq!(todos[2].title, "Task 1");
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_list_by_user_empty(pool: PgPool) -> Result<(), DomainError> {
    let user_id = create_test_user(&pool, "empty-todos@example.com").await?;

    let todos = TodoRepository::list_by_user(&pool, user_id).await?;

    assert!(todos.is_empty());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_list_by_user_isolates_users(pool: PgPool) -> Result<(), DomainError> {
    let user1 = create_test_user(&pool, "user1-todos@example.com").await?;
    let user2 = create_test_user(&pool, "user2-todos@example.com").await?;

    TodoRepository::create(&pool, user1, "User 1 Task", None).await?;
    TodoRepository::create(&pool, user2, "User 2 Task", None).await?;

    let user1_todos = TodoRepository::list_by_user(&pool, user1).await?;
    let user2_todos = TodoRepository::list_by_user(&pool, user2).await?;

    assert_eq!(user1_todos.len(), 1);
    assert_eq!(user1_todos[0].title, "User 1 Task");
    assert_eq!(user2_todos.len(), 1);
    assert_eq!(user2_todos[0].title, "User 2 Task");
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_list_by_user_and_status(pool: PgPool) -> Result<(), DomainError> {
    let user_id = create_test_user(&pool, "status-filter@example.com").await?;

    // Create todos and update some statuses
    let todo1 = TodoRepository::create(&pool, user_id, "Pending Task", None).await?;
    let todo2 = TodoRepository::create(&pool, user_id, "In Progress Task", None).await?;
    let todo3 = TodoRepository::create(&pool, user_id, "Completed Task", None).await?;

    TodoRepository::update_status(&pool, todo2.id, TodoStatus::InProgress).await?;
    TodoRepository::update_status(&pool, todo3.id, TodoStatus::Completed).await?;

    // Filter by status
    let pending =
        TodoRepository::list_by_user_and_status(&pool, user_id, TodoStatus::Pending).await?;
    let in_progress =
        TodoRepository::list_by_user_and_status(&pool, user_id, TodoStatus::InProgress).await?;
    let completed =
        TodoRepository::list_by_user_and_status(&pool, user_id, TodoStatus::Completed).await?;

    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, todo1.id);
    assert_eq!(in_progress.len(), 1);
    assert_eq!(in_progress[0].id, todo2.id);
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].id, todo3.id);
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_status(pool: PgPool) -> Result<(), DomainError> {
    let user_id = create_test_user(&pool, "update-status@example.com").await?;
    let created = TodoRepository::create(&pool, user_id, "Status Task", None).await?;

    assert_eq!(created.status, TodoStatus::Pending);

    // Update to InProgress
    let updated = TodoRepository::update_status(&pool, created.id, TodoStatus::InProgress).await?;
    assert!(updated.is_some());
    let updated = updated.unwrap();
    assert_eq!(updated.status, TodoStatus::InProgress);
    assert!(updated.updated_at > created.updated_at);

    // Update to Completed
    let completed = TodoRepository::update_status(&pool, created.id, TodoStatus::Completed).await?;
    assert!(completed.is_some());
    assert_eq!(completed.unwrap().status, TodoStatus::Completed);
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_status_not_found(pool: PgPool) -> Result<(), DomainError> {
    let updated =
        TodoRepository::update_status(&pool, Uuid::new_v4(), TodoStatus::Completed).await?;
    assert!(updated.is_none());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_content(pool: PgPool) -> Result<(), DomainError> {
    let user_id = create_test_user(&pool, "update-content@example.com").await?;
    let created =
        TodoRepository::create(&pool, user_id, "Original Title", Some("Original desc")).await?;

    let updated =
        TodoRepository::update_content(&pool, created.id, "New Title", Some("New description"))
            .await?;

    assert!(updated.is_some());
    let updated = updated.unwrap();
    assert_eq!(updated.title, "New Title");
    assert_eq!(updated.description, Some("New description".to_string()));
    assert!(updated.updated_at > created.updated_at);
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_content_remove_description(pool: PgPool) -> Result<(), DomainError> {
    let user_id = create_test_user(&pool, "remove-desc@example.com").await?;
    let created =
        TodoRepository::create(&pool, user_id, "Has Description", Some("Description")).await?;

    let updated = TodoRepository::update_content(&pool, created.id, "No Description", None).await?;

    assert!(updated.is_some());
    let updated = updated.unwrap();
    assert!(updated.description.is_none());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_content_not_found(pool: PgPool) -> Result<(), DomainError> {
    let updated = TodoRepository::update_content(&pool, Uuid::new_v4(), "Title", None).await?;
    assert!(updated.is_none());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_delete_todo(pool: PgPool) -> Result<(), DomainError> {
    let user_id = create_test_user(&pool, "delete-todo@example.com").await?;
    let created = TodoRepository::create(&pool, user_id, "Delete Me", None).await?;

    let deleted = TodoRepository::delete(&pool, created.id).await?;
    assert!(deleted);

    // Verify it's gone
    let found = TodoRepository::find_by_id(&pool, created.id).await?;
    assert!(found.is_none());
    Ok(())
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_delete_todo_not_found(pool: PgPool) -> Result<(), DomainError> {
    let deleted = TodoRepository::delete(&pool, Uuid::new_v4()).await?;
    assert!(!deleted);
    Ok(())
}
