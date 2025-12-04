//! BDD-style behavior tests for the Todo feature
//!
//! These tests verify todo-related business behaviors work correctly.
//! Focus on workflows and business rules, not implementation details.

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

// =============================================================================
// Todo Creation Behaviors
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn user_can_create_todo_with_title_and_description(
    pool: PgPool,
) -> Result<(), TodoFeatureError> {
    // Given a registered user
    let user_id = create_test_user(&pool, "create-todo@example.com").await;

    // When creating a todo with title and description
    let todo = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "My Task".to_string(),
            description: Some("A description".to_string()),
        },
    )
    .await?;

    // Then the todo is created with the correct data
    assert_eq!(todo.user_id, user_id);
    assert_eq!(todo.title, "My Task");
    assert_eq!(todo.description, Some("A description".to_string()));
    // And it starts in pending status
    assert_eq!(todo.status, TodoStatus::Pending);
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn user_can_create_todo_without_description(pool: PgPool) -> Result<(), TodoFeatureError> {
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
async fn creating_todo_for_nonexistent_user_fails(pool: PgPool) -> Result<(), TodoFeatureError> {
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

// =============================================================================
// Todo Query Behaviors
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn todo_can_be_found_by_id(pool: PgPool) -> Result<(), TodoFeatureError> {
    // Given a user with a todo
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

    // When querying by ID
    let found = TodoService::get(&pool, created.id).await?;

    // Then the todo is found
    assert_eq!(found.id, created.id);
    assert_eq!(found.title, "Find Me");
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn querying_nonexistent_todo_returns_not_found(pool: PgPool) -> Result<(), TodoFeatureError> {
    let result = TodoService::get(&pool, Uuid::new_v4()).await;

    assert!(matches!(result, Err(TodoFeatureError::NotFound(_))));
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn user_can_list_their_todos(pool: PgPool) -> Result<(), TodoFeatureError> {
    // Given a user with multiple todos
    let user_id = create_test_user(&pool, "list-todos@example.com").await;

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

    // When listing todos for the user
    let todos = TodoService::list_for_user(&pool, user_id).await?;

    // Then all todos are returned
    assert_eq!(todos.len(), 3);
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn listing_todos_when_none_exist_returns_empty(pool: PgPool) -> Result<(), TodoFeatureError> {
    let user_id = create_test_user(&pool, "empty-todos@example.com").await;

    let todos = TodoService::list_for_user(&pool, user_id).await?;

    assert!(todos.is_empty());
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn users_only_see_their_own_todos(pool: PgPool) -> Result<(), TodoFeatureError> {
    // Given two users with their own todos
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

    // When each user lists their todos
    let user1_todos = TodoService::list_for_user(&pool, user1).await?;
    let user2_todos = TodoService::list_for_user(&pool, user2).await?;

    // Then they only see their own
    assert_eq!(user1_todos.len(), 1);
    assert_eq!(user1_todos[0].title, "User 1 Task");
    assert_eq!(user2_todos.len(), 1);
    assert_eq!(user2_todos[0].title, "User 2 Task");
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn todos_can_be_filtered_by_status(pool: PgPool) -> Result<(), TodoFeatureError> {
    // Given a user with todos in different statuses
    let user_id = create_test_user(&pool, "status-list@example.com").await;

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

    // When filtering by each status
    let pending = TodoService::list_for_user_by_status(&pool, user_id, TodoStatus::Pending).await?;
    let in_progress =
        TodoService::list_for_user_by_status(&pool, user_id, TodoStatus::InProgress).await?;
    let completed =
        TodoService::list_for_user_by_status(&pool, user_id, TodoStatus::Completed).await?;

    // Then only matching todos are returned
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, todo1.id);
    assert_eq!(in_progress.len(), 1);
    assert_eq!(in_progress[0].id, todo2.id);
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].id, todo3.id);
    Ok(())
}

// =============================================================================
// Todo Update Behaviors
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn todo_title_can_be_updated(pool: PgPool) -> Result<(), TodoFeatureError> {
    // Given a user with a todo
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

    // When updating the title
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

    // Then the title is changed
    assert_eq!(updated.title, "New Title");
    // And the description is preserved
    assert_eq!(updated.description, Some("Original desc".to_string()));
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn todo_description_can_be_updated(pool: PgPool) -> Result<(), TodoFeatureError> {
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
async fn todo_status_can_be_updated_directly(pool: PgPool) -> Result<(), TodoFeatureError> {
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
async fn updating_nonexistent_todo_fails(pool: PgPool) -> Result<(), TodoFeatureError> {
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

// =============================================================================
// Todo Status Transition Behaviors
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn todo_can_be_marked_as_completed(pool: PgPool) -> Result<(), TodoFeatureError> {
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
async fn completing_nonexistent_todo_fails(pool: PgPool) -> Result<(), TodoFeatureError> {
    let result = TodoService::complete(&pool, Uuid::new_v4()).await;

    assert!(matches!(result, Err(TodoFeatureError::NotFound(_))));
    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn todo_can_be_started(pool: PgPool) -> Result<(), TodoFeatureError> {
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
async fn starting_nonexistent_todo_fails(pool: PgPool) -> Result<(), TodoFeatureError> {
    let result = TodoService::start(&pool, Uuid::new_v4()).await;

    assert!(matches!(result, Err(TodoFeatureError::NotFound(_))));
    Ok(())
}

// =============================================================================
// Todo Deletion Behaviors
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn todo_can_be_deleted(pool: PgPool) -> Result<(), TodoFeatureError> {
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
async fn deleting_nonexistent_todo_returns_false(pool: PgPool) -> Result<(), TodoFeatureError> {
    let deleted = TodoService::delete(&pool, Uuid::new_v4()).await?;
    assert!(!deleted);
    Ok(())
}
