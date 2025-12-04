//! Todo workflow tests - end-to-end todo workflows at the feature layer
//!
//! These tests verify complete todo workflows through the system.
//! They are transport-agnostic (no GraphQL, no HTTP).

use domain::TodoStatus;
use sqlx::PgPool;
use todo_feature::{CreateTodoInput, TodoFeatureError, TodoService, UpdateTodoInput};
use user_feature::{CreateUserInput, UserService};

/// Helper to create a test user
async fn create_user(pool: &PgPool, email: &str) -> uuid::Uuid {
    UserService::register(
        pool,
        CreateUserInput {
            email: email.to_string(),
            name: "Test User".to_string(),
        },
    )
    .await
    .expect("Failed to create user")
    .id
}

// =============================================================================
// Todo Lifecycle Workflow
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn todo_lifecycle_pending_to_completed(pool: PgPool) -> Result<(), TodoFeatureError> {
    // Given a user with a new todo
    let user_id = create_user(&pool, "lifecycle@example.com").await;
    let todo = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Lifecycle Task".to_string(),
            description: Some("Track this through its lifecycle".to_string()),
        },
    )
    .await?;

    // The todo starts as pending
    assert_eq!(todo.status, TodoStatus::Pending);

    // When the user starts working on it
    let in_progress = TodoService::start(&pool, todo.id).await?;
    assert_eq!(in_progress.status, TodoStatus::InProgress);

    // And then completes it
    let completed = TodoService::complete(&pool, todo.id).await?;
    assert_eq!(completed.status, TodoStatus::Completed);

    // The todo remains queryable with its final state
    let final_state = TodoService::get(&pool, todo.id).await?;
    assert_eq!(final_state.status, TodoStatus::Completed);
    assert_eq!(final_state.title, "Lifecycle Task");

    Ok(())
}

#[sqlx::test(migrations = "../../../migrations")]
async fn todo_can_be_completed_directly_from_pending(pool: PgPool) -> Result<(), TodoFeatureError> {
    // Some todos are completed immediately without going through in_progress
    let user_id = create_user(&pool, "direct-complete@example.com").await;
    let todo = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Quick Task".to_string(),
            description: None,
        },
    )
    .await?;

    // Complete directly from pending
    let completed = TodoService::complete(&pool, todo.id).await?;
    assert_eq!(completed.status, TodoStatus::Completed);

    Ok(())
}

// =============================================================================
// Task Management Workflow
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn user_manages_multiple_todos(pool: PgPool) -> Result<(), TodoFeatureError> {
    // Given a user who creates several todos
    let user_id = create_user(&pool, "multitask@example.com").await;

    let todo1 = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Task 1".to_string(),
            description: None,
        },
    )
    .await?;

    let todo2 = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Task 2".to_string(),
            description: None,
        },
    )
    .await?;

    let todo3 = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Task 3".to_string(),
            description: None,
        },
    )
    .await?;

    // All todos start as pending
    let all_pending = TodoService::list_for_user_by_status(&pool, user_id, TodoStatus::Pending).await?;
    assert_eq!(all_pending.len(), 3);

    // User starts working on task 1
    TodoService::start(&pool, todo1.id).await?;

    // Now we have 2 pending, 1 in progress
    let pending = TodoService::list_for_user_by_status(&pool, user_id, TodoStatus::Pending).await?;
    let in_progress =
        TodoService::list_for_user_by_status(&pool, user_id, TodoStatus::InProgress).await?;
    assert_eq!(pending.len(), 2);
    assert_eq!(in_progress.len(), 1);

    // User completes task 1 and starts task 2
    TodoService::complete(&pool, todo1.id).await?;
    TodoService::start(&pool, todo2.id).await?;

    // Now we have 1 pending, 1 in progress, 1 completed
    let pending = TodoService::list_for_user_by_status(&pool, user_id, TodoStatus::Pending).await?;
    let in_progress =
        TodoService::list_for_user_by_status(&pool, user_id, TodoStatus::InProgress).await?;
    let completed =
        TodoService::list_for_user_by_status(&pool, user_id, TodoStatus::Completed).await?;

    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, todo3.id);
    assert_eq!(in_progress.len(), 1);
    assert_eq!(in_progress[0].id, todo2.id);
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].id, todo1.id);

    Ok(())
}

// =============================================================================
// Todo Edit Workflow
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn todo_details_can_be_refined(pool: PgPool) -> Result<(), TodoFeatureError> {
    // Given a todo with minimal details
    let user_id = create_user(&pool, "refine@example.com").await;
    let todo = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id,
            title: "Vague Task".to_string(),
            description: None,
        },
    )
    .await?;

    // User can add a description later
    let with_desc = TodoService::update(
        &pool,
        todo.id,
        UpdateTodoInput {
            title: None,
            description: Some("Now with more details".to_string()),
            status: None,
        },
    )
    .await?;

    assert_eq!(with_desc.title, "Vague Task");
    assert_eq!(
        with_desc.description,
        Some("Now with more details".to_string())
    );

    // User can also clarify the title
    let refined = TodoService::update(
        &pool,
        todo.id,
        UpdateTodoInput {
            title: Some("Clear Task".to_string()),
            description: None,
            status: None,
        },
    )
    .await?;

    assert_eq!(refined.title, "Clear Task");
    // Description is preserved
    assert_eq!(
        refined.description,
        Some("Now with more details".to_string())
    );

    Ok(())
}

// =============================================================================
// User Isolation Workflow
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn users_todos_are_completely_isolated(pool: PgPool) -> Result<(), TodoFeatureError> {
    // Given two users each with their own todos
    let alice_id = create_user(&pool, "alice@example.com").await;
    let bob_id = create_user(&pool, "bob@example.com").await;

    // Alice creates her todos
    let alice_todo1 = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id: alice_id,
            title: "Alice Task 1".to_string(),
            description: None,
        },
    )
    .await?;

    let _alice_todo2 = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id: alice_id,
            title: "Alice Task 2".to_string(),
            description: None,
        },
    )
    .await?;

    // Bob creates his todo
    let bob_todo = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id: bob_id,
            title: "Bob Task".to_string(),
            description: None,
        },
    )
    .await?;

    // Alice only sees her todos
    let alice_todos = TodoService::list_for_user(&pool, alice_id).await?;
    assert_eq!(alice_todos.len(), 2);
    assert!(alice_todos.iter().all(|t| t.user_id == alice_id));

    // Bob only sees his todos
    let bob_todos = TodoService::list_for_user(&pool, bob_id).await?;
    assert_eq!(bob_todos.len(), 1);
    assert_eq!(bob_todos[0].id, bob_todo.id);

    // Alice's actions don't affect Bob's todos
    TodoService::complete(&pool, alice_todo1.id).await?;
    TodoService::delete(&pool, alice_todo1.id).await?;

    // Bob's todo is unaffected
    let bob_todo_after = TodoService::get(&pool, bob_todo.id).await?;
    assert_eq!(bob_todo_after.status, TodoStatus::Pending);

    Ok(())
}

// =============================================================================
// Complete User + Todo Workflow (Cross-Feature Integration)
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn complete_user_and_todo_workflow(pool: PgPool) -> Result<(), TodoFeatureError> {
    // 1. User registers
    let user = UserService::register(
        &pool,
        CreateUserInput {
            email: "complete@example.com".to_string(),
            name: "Complete User".to_string(),
        },
    )
    .await
    .expect("User registration failed");

    // 2. User creates todos
    let todo1 = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id: user.id,
            title: "First Task".to_string(),
            description: Some("My first todo".to_string()),
        },
    )
    .await?;

    let todo2 = TodoService::create(
        &pool,
        CreateTodoInput {
            user_id: user.id,
            title: "Second Task".to_string(),
            description: None,
        },
    )
    .await?;

    // 3. User works through their todos
    TodoService::start(&pool, todo1.id).await?;
    TodoService::complete(&pool, todo1.id).await?;

    // 4. User updates a todo
    TodoService::update(
        &pool,
        todo2.id,
        UpdateTodoInput {
            title: Some("Updated Second Task".to_string()),
            description: Some("Added description".to_string()),
            status: None,
        },
    )
    .await?;

    // 5. Verify final state
    let completed_todos =
        TodoService::list_for_user_by_status(&pool, user.id, TodoStatus::Completed).await?;
    assert_eq!(completed_todos.len(), 1);
    assert_eq!(completed_todos[0].title, "First Task");

    let pending_todos =
        TodoService::list_for_user_by_status(&pool, user.id, TodoStatus::Pending).await?;
    assert_eq!(pending_todos.len(), 1);
    assert_eq!(pending_todos[0].title, "Updated Second Task");
    assert_eq!(
        pending_todos[0].description,
        Some("Added description".to_string())
    );

    Ok(())
}
