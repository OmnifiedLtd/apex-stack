use domain::TodoStatus;
use sqlx::PgPool;
use todo_feature::{CreateTodoInput, TodoFeatureError, TodoService, UpdateTodoInput};
use user_feature::{CreateUserInput, UserService};
use uuid::Uuid;

/// Helper to create a test user (todos require a valid user)
async fn create_test_user(pool: &PgPool, email: &str) -> Uuid {
    let user = UserService::register(
        pool,
        CreateUserInput {
            email: email.to_string(),
            name: "Test User".to_string(),
        },
    )
    .await
    .expect("Failed to create test user");
    user.id
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_create_todo(pool: PgPool) -> Result<(), TodoFeatureError> {
    let user_id = create_test_user(&pool, "create-todo@example.com").await;

    let todo = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "My Task".to_string(),
            description: Some("A description".to_string()),
        },
    )
    .await?;

    assert_eq!(todo.user_id, user_id);
    assert_eq!(todo.title, "My Task");
    assert_eq!(todo.description, Some("A description".to_string()));
    assert_eq!(todo.status, TodoStatus::Pending);
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_create_todo_without_description(pool: PgPool) -> Result<(), TodoFeatureError> {
    let user_id = create_test_user(&pool, "create-todo2@example.com").await;

    let todo = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Simple Task".to_string(),
            description: None,
        },
    )
    .await?;

    assert_eq!(todo.title, "Simple Task");
    assert!(todo.description.is_none());
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_create_todo_user_not_found(pool: PgPool) -> Result<(), TodoFeatureError> {
    let result = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id: Uuid::new_v4(),
            title: "Task".to_string(),
            description: None,
        },
    )
    .await;

    assert!(matches!(result, Err(TodoFeatureError::UserNotFound(_))));
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_get_todo(pool: PgPool) -> Result<(), TodoFeatureError> {
    let user_id = create_test_user(&pool, "get-todo@example.com").await;

    let created = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Find Me".to_string(),
            description: None,
        },
    )
    .await?;

    let found = TodoService::get(&pool, created.id).await?;

    assert_eq!(found.id, created.id);
    assert_eq!(found.title, "Find Me");
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_get_todo_not_found(pool: PgPool) -> Result<(), TodoFeatureError> {
    let result = TodoService::get(&pool, Uuid::new_v4()).await;

    assert!(matches!(result, Err(TodoFeatureError::NotFound(_))));
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_list_for_user(pool: PgPool) -> Result<(), TodoFeatureError> {
    let user_id = create_test_user(&pool, "list-todos@example.com").await;

    // Create multiple todos
    TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Task 1".to_string(),
            description: None,
        },
    )
    .await?;

    TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Task 2".to_string(),
            description: None,
        },
    )
    .await?;

    TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Task 3".to_string(),
            description: None,
        },
    )
    .await?;

    let todos = TodoService::list_for_user(&pool, user_id).await?;

    assert_eq!(todos.len(), 3);
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_list_for_user_empty(pool: PgPool) -> Result<(), TodoFeatureError> {
    let user_id = create_test_user(&pool, "empty-todos@example.com").await;

    let todos = TodoService::list_for_user(&pool, user_id).await?;

    assert!(todos.is_empty());
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_list_for_user_isolates_users(pool: PgPool) -> Result<(), TodoFeatureError> {
    let user1 = create_test_user(&pool, "user1@example.com").await;
    let user2 = create_test_user(&pool, "user2@example.com").await;

    TodoService::create(
        &pool,
        CreateTodoInput {
            user_id: user1,
            title: "User 1 Task".to_string(),
            description: None,
        },
    )
    .await?;

    TodoService::create(
        &pool,
        CreateTodoInput {
            user_id: user2,
            title: "User 2 Task".to_string(),
            description: None,
        },
    )
    .await?;

    let user1_todos = TodoService::list_for_user(&pool, user1).await?;
    let user2_todos = TodoService::list_for_user(&pool, user2).await?;

    assert_eq!(user1_todos.len(), 1);
    assert_eq!(user1_todos[0].title, "User 1 Task");
    assert_eq!(user2_todos.len(), 1);
    assert_eq!(user2_todos[0].title, "User 2 Task");
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_list_for_user_by_status(pool: PgPool) -> Result<(), TodoFeatureError> {
    let user_id = create_test_user(&pool, "status-list@example.com").await;

    // Create todos with different statuses
    let todo1 = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Pending".to_string(),
            description: None,
        },
    )
    .await?;

    let todo2 = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "In Progress".to_string(),
            description: None,
        },
    )
    .await?;
    TodoService::start(&pool, todo2.id).await?;

    let todo3 = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Completed".to_string(),
            description: None,
        },
    )
    .await?;
    TodoService::complete(&pool, todo3.id).await?;

    // Filter by status
    let pending = TodoService::list_for_user_by_status(&pool, user_id, TodoStatus::Pending).await?;
    let in_progress =
        TodoService::list_for_user_by_status(&pool, user_id, TodoStatus::InProgress).await?;
    let completed =
        TodoService::list_for_user_by_status(&pool, user_id, TodoStatus::Completed).await?;

    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, todo1.id);
    assert_eq!(in_progress.len(), 1);
    assert_eq!(in_progress[0].id, todo2.id);
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].id, todo3.id);
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_update_todo_title(pool: PgPool) -> Result<(), TodoFeatureError> {
    let user_id = create_test_user(&pool, "update-title@example.com").await;

    let created = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Original".to_string(),
            description: Some("Original desc".to_string()),
        },
    )
    .await?;

    let updated = TodoService::update(
        &pool,
        created.id,
        UpdateTodoInput {
            title: Some("New Title".to_string()),
            description: None,
            status: None,
        },
    )
    .await?;

    assert_eq!(updated.title, "New Title");
    // Description should be preserved
    assert_eq!(updated.description, Some("Original desc".to_string()));
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_update_todo_description(pool: PgPool) -> Result<(), TodoFeatureError> {
    let user_id = create_test_user(&pool, "update-desc@example.com").await;

    let created = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Title".to_string(),
            description: Some("Old".to_string()),
        },
    )
    .await?;

    let updated = TodoService::update(
        &pool,
        created.id,
        UpdateTodoInput {
            title: None,
            description: Some("New description".to_string()),
            status: None,
        },
    )
    .await?;

    assert_eq!(updated.title, "Title"); // Unchanged
    assert_eq!(updated.description, Some("New description".to_string()));
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_update_todo_status(pool: PgPool) -> Result<(), TodoFeatureError> {
    let user_id = create_test_user(&pool, "update-status@example.com").await;

    let created = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Status Task".to_string(),
            description: None,
        },
    )
    .await?;

    assert_eq!(created.status, TodoStatus::Pending);

    let updated = TodoService::update(
        &pool,
        created.id,
        UpdateTodoInput {
            title: None,
            description: None,
            status: Some(TodoStatus::InProgress),
        },
    )
    .await?;

    assert_eq!(updated.status, TodoStatus::InProgress);
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_update_todo_not_found(pool: PgPool) -> Result<(), TodoFeatureError> {
    let result = TodoService::update(
        &pool,
        Uuid::new_v4(),
        UpdateTodoInput {
            title: Some("New".to_string()),
            description: None,
            status: None,
        },
    )
    .await;

    assert!(matches!(result, Err(TodoFeatureError::NotFound(_))));
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_complete_todo(pool: PgPool) -> Result<(), TodoFeatureError> {
    let user_id = create_test_user(&pool, "complete@example.com").await;

    let created = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Complete Me".to_string(),
            description: None,
        },
    )
    .await?;

    let completed = TodoService::complete(&pool, created.id).await?;

    assert_eq!(completed.status, TodoStatus::Completed);
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_complete_todo_not_found(pool: PgPool) -> Result<(), TodoFeatureError> {
    let result = TodoService::complete(&pool, Uuid::new_v4()).await;

    assert!(matches!(result, Err(TodoFeatureError::NotFound(_))));
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_start_todo(pool: PgPool) -> Result<(), TodoFeatureError> {
    let user_id = create_test_user(&pool, "start@example.com").await;

    let created = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Start Me".to_string(),
            description: None,
        },
    )
    .await?;

    let started = TodoService::start(&pool, created.id).await?;

    assert_eq!(started.status, TodoStatus::InProgress);
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_start_todo_not_found(pool: PgPool) -> Result<(), TodoFeatureError> {
    let result = TodoService::start(&pool, Uuid::new_v4()).await;

    assert!(matches!(result, Err(TodoFeatureError::NotFound(_))));
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_delete_todo(pool: PgPool) -> Result<(), TodoFeatureError> {
    let user_id = create_test_user(&pool, "delete@example.com").await;

    let created = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Delete Me".to_string(),
            description: None,
        },
    )
    .await?;

    let deleted = TodoService::delete(&pool, created.id).await?;
    assert!(deleted);

    // Verify todo is gone
    let result = TodoService::get(&pool, created.id).await;
    assert!(matches!(result, Err(TodoFeatureError::NotFound(_))));
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn test_delete_todo_not_found(pool: PgPool) -> Result<(), TodoFeatureError> {
    let deleted = TodoService::delete(&pool, Uuid::new_v4()).await?;
    assert!(!deleted);
    Ok(())
}
