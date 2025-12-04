//! BDD-style integration tests for the GraphQL API
//!
//! These tests focus on behavior verification rather than exact response shapes.
//! They are designed to be resilient to schema changes like adding new fields.
//!
//! Key principles:
//! - Only assert on fields that are relevant to the behavior being tested
//! - Use serde_json::Value for flexible response handling
//! - Extract IDs dynamically for use in subsequent operations
//! - Test user journeys, not individual fields

use async_graphql::Request;
use graphql_api::build_schema;
use serde_json::Value;
use sqlx::PgPool;

/// Helper to execute a GraphQL query and return the response as JSON
async fn execute(pool: &PgPool, query: &str) -> Value {
    let schema = build_schema(pool.clone());
    let response = schema.execute(Request::new(query)).await;
    serde_json::to_value(&response).expect("Failed to serialize response")
}

/// Assert that a response has no errors
fn assert_no_errors(response: &Value) {
    let errors = &response["errors"];
    assert!(
        errors.is_null() || errors.as_array().map(|a| a.is_empty()).unwrap_or(true),
        "Expected no errors, got: {}",
        serde_json::to_string_pretty(errors).unwrap()
    );
}

/// Assert that a response has errors (for negative test cases)
fn assert_has_errors(response: &Value) {
    let errors = &response["errors"];
    assert!(
        errors.is_array() && !errors.as_array().unwrap().is_empty(),
        "Expected errors but got none"
    );
}

/// Extract a string field from a JSON path
fn get_string(value: &Value, path: &[&str]) -> String {
    let mut current = value;
    for key in path {
        current = &current[*key];
    }
    current.as_str().expect("Expected string value").to_string()
}

// =============================================================================
// User Registration Scenarios
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn user_can_register_with_email_and_name(pool: PgPool) {
    // When I register a new user
    let response = execute(
        &pool,
        r#"
        mutation {
            registerUser(input: { email: "alice@example.com", name: "Alice" }) {
                id
                email
                name
            }
        }
        "#,
    )
    .await;

    // Then the registration succeeds
    assert_no_errors(&response);

    // And the user has the correct email and name
    let user = &response["data"]["registerUser"];
    assert_eq!(user["email"], "alice@example.com");
    assert_eq!(user["name"], "Alice");

    // And an ID is assigned
    assert!(user["id"].is_string(), "User should have an ID");
}

#[sqlx::test(migrations = "../../../migrations")]
async fn registered_user_can_be_queried_by_id(pool: PgPool) {
    // Given a registered user
    let register_response = execute(
        &pool,
        r#"
        mutation {
            registerUser(input: { email: "bob@example.com", name: "Bob" }) {
                id
            }
        }
        "#,
    )
    .await;
    let user_id = get_string(&register_response, &["data", "registerUser", "id"]);

    // When I query for the user by ID
    let query = format!(
        r#"
        query {{
            user(id: "{}") {{
                email
                name
            }}
        }}
        "#,
        user_id
    );
    let response = execute(&pool, &query).await;

    // Then the user is found
    assert_no_errors(&response);
    let user = &response["data"]["user"];
    assert_eq!(user["email"], "bob@example.com");
    assert_eq!(user["name"], "Bob");
}

#[sqlx::test(migrations = "../../../migrations")]
async fn registered_user_can_be_queried_by_email(pool: PgPool) {
    // Given a registered user
    execute(
        &pool,
        r#"
        mutation {
            registerUser(input: { email: "charlie@example.com", name: "Charlie" }) {
                id
            }
        }
        "#,
    )
    .await;

    // When I query for the user by email
    let response = execute(
        &pool,
        r#"
        query {
            userByEmail(email: "charlie@example.com") {
                name
            }
        }
        "#,
    )
    .await;

    // Then the user is found
    assert_no_errors(&response);
    assert_eq!(response["data"]["userByEmail"]["name"], "Charlie");
}

#[sqlx::test(migrations = "../../../migrations")]
async fn duplicate_email_registration_fails(pool: PgPool) {
    // Given a user with a specific email
    execute(
        &pool,
        r#"
        mutation {
            registerUser(input: { email: "taken@example.com", name: "First" }) {
                id
            }
        }
        "#,
    )
    .await;

    // When I try to register another user with the same email
    let response = execute(
        &pool,
        r#"
        mutation {
            registerUser(input: { email: "taken@example.com", name: "Second" }) {
                id
            }
        }
        "#,
    )
    .await;

    // Then the registration fails with an error
    assert_has_errors(&response);
}

#[sqlx::test(migrations = "../../../migrations")]
async fn user_name_can_be_updated(pool: PgPool) {
    // Given a registered user
    let register_response = execute(
        &pool,
        r#"
        mutation {
            registerUser(input: { email: "update@example.com", name: "Original" }) {
                id
            }
        }
        "#,
    )
    .await;
    let user_id = get_string(&register_response, &["data", "registerUser", "id"]);

    // When I update the user's name
    let query = format!(
        r#"
        mutation {{
            updateUser(id: "{}", input: {{ name: "Updated" }}) {{
                name
                email
            }}
        }}
        "#,
        user_id
    );
    let response = execute(&pool, &query).await;

    // Then the name is updated
    assert_no_errors(&response);
    assert_eq!(response["data"]["updateUser"]["name"], "Updated");
    // And the email remains unchanged
    assert_eq!(response["data"]["updateUser"]["email"], "update@example.com");
}

#[sqlx::test(migrations = "../../../migrations")]
async fn user_can_be_deleted(pool: PgPool) {
    // Given a registered user
    let register_response = execute(
        &pool,
        r#"
        mutation {
            registerUser(input: { email: "delete@example.com", name: "ToDelete" }) {
                id
            }
        }
        "#,
    )
    .await;
    let user_id = get_string(&register_response, &["data", "registerUser", "id"]);

    // When I delete the user
    let delete_query = format!(
        r#"
        mutation {{
            deleteUser(id: "{}")
        }}
        "#,
        user_id
    );
    let delete_response = execute(&pool, &delete_query).await;

    // Then the deletion succeeds
    assert_no_errors(&delete_response);
    assert_eq!(delete_response["data"]["deleteUser"], true);

    // And the user no longer exists
    let query = format!(
        r#"
        query {{
            user(id: "{}") {{
                id
            }}
        }}
        "#,
        user_id
    );
    let response = execute(&pool, &query).await;
    assert_no_errors(&response);
    assert!(response["data"]["user"].is_null());
}

#[sqlx::test(migrations = "../../../migrations")]
async fn users_can_be_listed(pool: PgPool) {
    // Given multiple registered users
    execute(
        &pool,
        r#"mutation { registerUser(input: { email: "list1@example.com", name: "User1" }) { id } }"#,
    )
    .await;
    execute(
        &pool,
        r#"mutation { registerUser(input: { email: "list2@example.com", name: "User2" }) { id } }"#,
    )
    .await;

    // When I list all users
    let response = execute(
        &pool,
        r#"
        query {
            users {
                email
            }
        }
        "#,
    )
    .await;

    // Then both users appear in the list
    assert_no_errors(&response);
    let users = response["data"]["users"].as_array().expect("Expected array");
    assert_eq!(users.len(), 2);
}

// =============================================================================
// Todo Workflow Scenarios
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn user_can_create_todo(pool: PgPool) {
    // Given a registered user
    let register_response = execute(
        &pool,
        r#"mutation { registerUser(input: { email: "todo@example.com", name: "TodoUser" }) { id } }"#,
    )
    .await;
    let user_id = get_string(&register_response, &["data", "registerUser", "id"]);

    // When I create a todo for the user
    let query = format!(
        r#"
        mutation {{
            createTodo(input: {{
                userId: "{}",
                title: "My first task",
                description: "This is a description"
            }}) {{
                id
                title
                description
                status
            }}
        }}
        "#,
        user_id
    );
    let response = execute(&pool, &query).await;

    // Then the todo is created
    assert_no_errors(&response);
    let todo = &response["data"]["createTodo"];
    assert_eq!(todo["title"], "My first task");
    assert_eq!(todo["description"], "This is a description");
    assert_eq!(todo["status"], "PENDING"); // New todos start as pending
}

#[sqlx::test(migrations = "../../../migrations")]
async fn todo_can_be_created_without_description(pool: PgPool) {
    // Given a registered user
    let register_response = execute(
        &pool,
        r#"mutation { registerUser(input: { email: "nodesc@example.com", name: "NoDesc" }) { id } }"#,
    )
    .await;
    let user_id = get_string(&register_response, &["data", "registerUser", "id"]);

    // When I create a todo without a description
    let query = format!(
        r#"
        mutation {{
            createTodo(input: {{ userId: "{}", title: "Simple task" }}) {{
                title
                description
            }}
        }}
        "#,
        user_id
    );
    let response = execute(&pool, &query).await;

    // Then the todo is created with null description
    assert_no_errors(&response);
    assert_eq!(response["data"]["createTodo"]["title"], "Simple task");
    assert!(response["data"]["createTodo"]["description"].is_null());
}

#[sqlx::test(migrations = "../../../migrations")]
async fn creating_todo_for_nonexistent_user_fails(pool: PgPool) {
    // When I try to create a todo for a non-existent user
    let response = execute(
        &pool,
        r#"
        mutation {
            createTodo(input: {
                userId: "00000000-0000-0000-0000-000000000000",
                title: "Orphan task"
            }) {
                id
            }
        }
        "#,
    )
    .await;

    // Then the operation fails
    assert_has_errors(&response);
}

#[sqlx::test(migrations = "../../../migrations")]
async fn todo_workflow_pending_to_in_progress_to_completed(pool: PgPool) {
    // Given a user with a todo
    let register_response = execute(
        &pool,
        r#"mutation { registerUser(input: { email: "workflow@example.com", name: "Worker" }) { id } }"#,
    )
    .await;
    let user_id = get_string(&register_response, &["data", "registerUser", "id"]);

    let create_response = execute(
        &pool,
        &format!(
            r#"mutation {{ createTodo(input: {{ userId: "{}", title: "Workflow task" }}) {{ id status }} }}"#,
            user_id
        ),
    )
    .await;
    let todo_id = get_string(&create_response, &["data", "createTodo", "id"]);
    assert_eq!(create_response["data"]["createTodo"]["status"], "PENDING");

    // When I start the todo
    let start_response = execute(
        &pool,
        &format!(r#"mutation {{ startTodo(id: "{}") {{ status }} }}"#, todo_id),
    )
    .await;

    // Then it becomes in progress
    assert_no_errors(&start_response);
    assert_eq!(start_response["data"]["startTodo"]["status"], "IN_PROGRESS");

    // When I complete the todo
    let complete_response = execute(
        &pool,
        &format!(
            r#"mutation {{ completeTodo(id: "{}") {{ status }} }}"#,
            todo_id
        ),
    )
    .await;

    // Then it becomes completed
    assert_no_errors(&complete_response);
    assert_eq!(
        complete_response["data"]["completeTodo"]["status"],
        "COMPLETED"
    );
}

#[sqlx::test(migrations = "../../../migrations")]
async fn todos_can_be_listed_for_user(pool: PgPool) {
    // Given a user with multiple todos
    let register_response = execute(
        &pool,
        r#"mutation { registerUser(input: { email: "listtodos@example.com", name: "Lister" }) { id } }"#,
    )
    .await;
    let user_id = get_string(&register_response, &["data", "registerUser", "id"]);

    execute(
        &pool,
        &format!(
            r#"mutation {{ createTodo(input: {{ userId: "{}", title: "Task 1" }}) {{ id }} }}"#,
            user_id
        ),
    )
    .await;
    execute(
        &pool,
        &format!(
            r#"mutation {{ createTodo(input: {{ userId: "{}", title: "Task 2" }}) {{ id }} }}"#,
            user_id
        ),
    )
    .await;
    execute(
        &pool,
        &format!(
            r#"mutation {{ createTodo(input: {{ userId: "{}", title: "Task 3" }}) {{ id }} }}"#,
            user_id
        ),
    )
    .await;

    // When I list todos for the user
    let response = execute(
        &pool,
        &format!(
            r#"query {{ todosForUser(userId: "{}") {{ title }} }}"#,
            user_id
        ),
    )
    .await;

    // Then all todos are returned
    assert_no_errors(&response);
    let todos = response["data"]["todosForUser"]
        .as_array()
        .expect("Expected array");
    assert_eq!(todos.len(), 3);
}

#[sqlx::test(migrations = "../../../migrations")]
async fn todos_can_be_filtered_by_status(pool: PgPool) {
    // Given a user with todos in different statuses
    let register_response = execute(
        &pool,
        r#"mutation { registerUser(input: { email: "filter@example.com", name: "Filterer" }) { id } }"#,
    )
    .await;
    let user_id = get_string(&register_response, &["data", "registerUser", "id"]);

    // Create and leave as pending
    execute(
        &pool,
        &format!(
            r#"mutation {{ createTodo(input: {{ userId: "{}", title: "Pending task" }}) {{ id }} }}"#,
            user_id
        ),
    )
    .await;

    // Create and start
    let started = execute(
        &pool,
        &format!(
            r#"mutation {{ createTodo(input: {{ userId: "{}", title: "Started task" }}) {{ id }} }}"#,
            user_id
        ),
    )
    .await;
    let started_id = get_string(&started, &["data", "createTodo", "id"]);
    execute(
        &pool,
        &format!(r#"mutation {{ startTodo(id: "{}") {{ id }} }}"#, started_id),
    )
    .await;

    // Create and complete
    let completed = execute(
        &pool,
        &format!(
            r#"mutation {{ createTodo(input: {{ userId: "{}", title: "Done task" }}) {{ id }} }}"#,
            user_id
        ),
    )
    .await;
    let completed_id = get_string(&completed, &["data", "createTodo", "id"]);
    execute(
        &pool,
        &format!(
            r#"mutation {{ completeTodo(id: "{}") {{ id }} }}"#,
            completed_id
        ),
    )
    .await;

    // When I filter by PENDING status
    let pending_response = execute(
        &pool,
        &format!(
            r#"query {{ todosForUserByStatus(userId: "{}", status: PENDING) {{ title }} }}"#,
            user_id
        ),
    )
    .await;

    // Then only pending todos are returned
    assert_no_errors(&pending_response);
    let pending = pending_response["data"]["todosForUserByStatus"]
        .as_array()
        .unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0]["title"], "Pending task");

    // When I filter by IN_PROGRESS status
    let in_progress_response = execute(
        &pool,
        &format!(
            r#"query {{ todosForUserByStatus(userId: "{}", status: IN_PROGRESS) {{ title }} }}"#,
            user_id
        ),
    )
    .await;

    // Then only in-progress todos are returned
    assert_no_errors(&in_progress_response);
    let in_progress = in_progress_response["data"]["todosForUserByStatus"]
        .as_array()
        .unwrap();
    assert_eq!(in_progress.len(), 1);
    assert_eq!(in_progress[0]["title"], "Started task");
}

#[sqlx::test(migrations = "../../../migrations")]
async fn todo_can_be_updated(pool: PgPool) {
    // Given a user with a todo
    let register_response = execute(
        &pool,
        r#"mutation { registerUser(input: { email: "updatetodo@example.com", name: "Updater" }) { id } }"#,
    )
    .await;
    let user_id = get_string(&register_response, &["data", "registerUser", "id"]);

    let create_response = execute(
        &pool,
        &format!(
            r#"mutation {{ createTodo(input: {{ userId: "{}", title: "Original", description: "Old desc" }}) {{ id }} }}"#,
            user_id
        ),
    )
    .await;
    let todo_id = get_string(&create_response, &["data", "createTodo", "id"]);

    // When I update the todo's title and description
    let update_response = execute(
        &pool,
        &format!(
            r#"mutation {{ updateTodo(id: "{}", input: {{ title: "Updated", description: "New desc" }}) {{ title description }} }}"#,
            todo_id
        ),
    )
    .await;

    // Then the fields are updated
    assert_no_errors(&update_response);
    assert_eq!(update_response["data"]["updateTodo"]["title"], "Updated");
    assert_eq!(
        update_response["data"]["updateTodo"]["description"],
        "New desc"
    );
}

#[sqlx::test(migrations = "../../../migrations")]
async fn todo_can_be_deleted(pool: PgPool) {
    // Given a user with a todo
    let register_response = execute(
        &pool,
        r#"mutation { registerUser(input: { email: "deletetodo@example.com", name: "Deleter" }) { id } }"#,
    )
    .await;
    let user_id = get_string(&register_response, &["data", "registerUser", "id"]);

    let create_response = execute(
        &pool,
        &format!(
            r#"mutation {{ createTodo(input: {{ userId: "{}", title: "To delete" }}) {{ id }} }}"#,
            user_id
        ),
    )
    .await;
    let todo_id = get_string(&create_response, &["data", "createTodo", "id"]);

    // When I delete the todo
    let delete_response = execute(
        &pool,
        &format!(r#"mutation {{ deleteTodo(id: "{}") }}"#, todo_id),
    )
    .await;

    // Then deletion succeeds
    assert_no_errors(&delete_response);
    assert_eq!(delete_response["data"]["deleteTodo"], true);

    // And the todo no longer exists
    let query_response = execute(
        &pool,
        &format!(r#"query {{ todo(id: "{}") {{ id }} }}"#, todo_id),
    )
    .await;
    assert_no_errors(&query_response);
    assert!(query_response["data"]["todo"].is_null());
}

// =============================================================================
// Data Isolation Scenarios
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn users_only_see_their_own_todos(pool: PgPool) {
    // Given two users with their own todos
    let user1_response = execute(
        &pool,
        r#"mutation { registerUser(input: { email: "user1@example.com", name: "User1" }) { id } }"#,
    )
    .await;
    let user1_id = get_string(&user1_response, &["data", "registerUser", "id"]);

    let user2_response = execute(
        &pool,
        r#"mutation { registerUser(input: { email: "user2@example.com", name: "User2" }) { id } }"#,
    )
    .await;
    let user2_id = get_string(&user2_response, &["data", "registerUser", "id"]);

    execute(
        &pool,
        &format!(
            r#"mutation {{ createTodo(input: {{ userId: "{}", title: "User1 task" }}) {{ id }} }}"#,
            user1_id
        ),
    )
    .await;
    execute(
        &pool,
        &format!(
            r#"mutation {{ createTodo(input: {{ userId: "{}", title: "User2 task" }}) {{ id }} }}"#,
            user2_id
        ),
    )
    .await;

    // When user1 lists their todos
    let user1_todos = execute(
        &pool,
        &format!(
            r#"query {{ todosForUser(userId: "{}") {{ title }} }}"#,
            user1_id
        ),
    )
    .await;

    // Then they only see their own todo
    assert_no_errors(&user1_todos);
    let todos = user1_todos["data"]["todosForUser"].as_array().unwrap();
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0]["title"], "User1 task");
}
