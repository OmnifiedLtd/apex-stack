//! GraphQL API smoke tests
//!
//! These are end-to-end tests that verify the API works as a whole.
//! They test happy paths and catch integration issues between layers.
//!
//! Keep these minimal - detailed behavior testing happens at the feature layer.

use async_graphql::Request;
use graphql_api::build_schema;
use serde_json::Value;
use sqlx::PgPool;

/// Execute a GraphQL query and return the response as JSON
async fn execute(pool: &PgPool, query: &str) -> Value {
    let schema = build_schema(pool.clone());
    let response = schema.execute(Request::new(query)).await;
    serde_json::to_value(&response).expect("Failed to serialize response")
}

/// Assert response has no errors
fn assert_no_errors(response: &Value) {
    let errors = &response["errors"];
    assert!(
        errors.is_null() || errors.as_array().map(|a| a.is_empty()).unwrap_or(true),
        "Expected no errors, got: {}",
        serde_json::to_string_pretty(errors).unwrap()
    );
}

/// Extract a string value from a JSON path
fn get_string(value: &Value, path: &[&str]) -> String {
    let mut current = value;
    for key in path {
        current = &current[*key];
    }
    current.as_str().expect("Expected string").to_string()
}

// =============================================================================
// Smoke Tests
// =============================================================================

#[sqlx::test(migrations = "../../../migrations")]
async fn smoke_test_complete_user_and_todo_workflow(pool: PgPool) {
    // This test verifies that all layers work together correctly:
    // GraphQL -> Features -> Domain -> Database

    // 1. Register a user
    let register_response = execute(
        &pool,
        r#"
        mutation {
            registerUser(input: { email: "smoke@test.com", name: "Smoke Test User" }) {
                id
                email
            }
        }
        "#,
    )
    .await;
    assert_no_errors(&register_response);
    let user_id = get_string(&register_response, &["data", "registerUser", "id"]);
    assert_eq!(
        register_response["data"]["registerUser"]["email"],
        "smoke@test.com"
    );

    // 2. Query the user back
    let user_query = format!(r#"query {{ user(id: "{}") {{ name }} }}"#, user_id);
    let user_response = execute(&pool, &user_query).await;
    assert_no_errors(&user_response);
    assert_eq!(user_response["data"]["user"]["name"], "Smoke Test User");

    // 3. Create a todo for the user
    let create_todo_query = format!(
        r#"
        mutation {{
            createTodo(input: {{
                userId: "{}",
                title: "Smoke Test Todo",
                description: "Testing the whole stack"
            }}) {{
                id
                title
                status
            }}
        }}
        "#,
        user_id
    );
    let todo_response = execute(&pool, &create_todo_query).await;
    assert_no_errors(&todo_response);
    let todo_id = get_string(&todo_response, &["data", "createTodo", "id"]);
    assert_eq!(
        todo_response["data"]["createTodo"]["status"],
        "PENDING"
    );

    // 4. Start the todo
    let start_query = format!(
        r#"mutation {{ startTodo(id: "{}") {{ status }} }}"#,
        todo_id
    );
    let start_response = execute(&pool, &start_query).await;
    assert_no_errors(&start_response);
    assert_eq!(
        start_response["data"]["startTodo"]["status"],
        "IN_PROGRESS"
    );

    // 5. Complete the todo
    let complete_query = format!(
        r#"mutation {{ completeTodo(id: "{}") {{ status }} }}"#,
        todo_id
    );
    let complete_response = execute(&pool, &complete_query).await;
    assert_no_errors(&complete_response);
    assert_eq!(
        complete_response["data"]["completeTodo"]["status"],
        "COMPLETED"
    );

    // 6. List todos for user and verify
    let list_query = format!(
        r#"query {{ todosForUser(userId: "{}") {{ title status }} }}"#,
        user_id
    );
    let list_response = execute(&pool, &list_query).await;
    assert_no_errors(&list_response);
    let todos = list_response["data"]["todosForUser"].as_array().unwrap();
    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0]["status"], "COMPLETED");
}

#[sqlx::test(migrations = "../../../migrations")]
async fn smoke_test_error_handling(pool: PgPool) {
    // Verify that errors from the feature layer are properly propagated as GraphQL errors

    // Try to create a todo for non-existent user
    let response = execute(
        &pool,
        r#"
        mutation {
            createTodo(input: {
                userId: "00000000-0000-0000-0000-000000000000",
                title: "Should Fail"
            }) {
                id
            }
        }
        "#,
    )
    .await;

    // Should have errors
    let errors = &response["errors"];
    assert!(errors.is_array());
    assert!(!errors.as_array().unwrap().is_empty());

    // Error should have a message
    let error_message = errors[0]["message"].as_str().unwrap();
    assert!(!error_message.is_empty());
}

#[sqlx::test(migrations = "../../../migrations")]
async fn smoke_test_user_crud(pool: PgPool) {
    // Create
    let create_response = execute(
        &pool,
        r#"mutation { registerUser(input: { email: "crud@test.com", name: "CRUD User" }) { id } }"#,
    )
    .await;
    assert_no_errors(&create_response);
    let user_id = get_string(&create_response, &["data", "registerUser", "id"]);

    // Read
    let read_response = execute(
        &pool,
        &format!(r#"query {{ user(id: "{}") {{ name }} }}"#, user_id),
    )
    .await;
    assert_no_errors(&read_response);
    assert_eq!(read_response["data"]["user"]["name"], "CRUD User");

    // Update
    let update_response = execute(
        &pool,
        &format!(
            r#"mutation {{ updateUser(id: "{}", input: {{ name: "Updated User" }}) {{ name }} }}"#,
            user_id
        ),
    )
    .await;
    assert_no_errors(&update_response);
    assert_eq!(
        update_response["data"]["updateUser"]["name"],
        "Updated User"
    );

    // Delete
    let delete_response = execute(
        &pool,
        &format!(r#"mutation {{ deleteUser(id: "{}") }}"#, user_id),
    )
    .await;
    assert_no_errors(&delete_response);
    assert_eq!(delete_response["data"]["deleteUser"], true);

    // Verify deleted
    let verify_response = execute(
        &pool,
        &format!(r#"query {{ user(id: "{}") {{ id }} }}"#, user_id),
    )
    .await;
    assert_no_errors(&verify_response);
    assert!(verify_response["data"]["user"].is_null());
}

#[sqlx::test(migrations = "../../../migrations")]
async fn smoke_test_todo_crud(pool: PgPool) {
    // Setup: create user
    let user_response = execute(
        &pool,
        r#"mutation { registerUser(input: { email: "todo-crud@test.com", name: "User" }) { id } }"#,
    )
    .await;
    let user_id = get_string(&user_response, &["data", "registerUser", "id"]);

    // Create
    let create_response = execute(
        &pool,
        &format!(
            r#"mutation {{ createTodo(input: {{ userId: "{}", title: "CRUD Todo" }}) {{ id title }} }}"#,
            user_id
        ),
    )
    .await;
    assert_no_errors(&create_response);
    let todo_id = get_string(&create_response, &["data", "createTodo", "id"]);
    assert_eq!(create_response["data"]["createTodo"]["title"], "CRUD Todo");

    // Read
    let read_response = execute(
        &pool,
        &format!(r#"query {{ todo(id: "{}") {{ title }} }}"#, todo_id),
    )
    .await;
    assert_no_errors(&read_response);
    assert_eq!(read_response["data"]["todo"]["title"], "CRUD Todo");

    // Update
    let update_response = execute(
        &pool,
        &format!(
            r#"mutation {{ updateTodo(id: "{}", input: {{ title: "Updated Todo" }}) {{ title }} }}"#,
            todo_id
        ),
    )
    .await;
    assert_no_errors(&update_response);
    assert_eq!(
        update_response["data"]["updateTodo"]["title"],
        "Updated Todo"
    );

    // Delete
    let delete_response = execute(
        &pool,
        &format!(r#"mutation {{ deleteTodo(id: "{}") }}"#, todo_id),
    )
    .await;
    assert_no_errors(&delete_response);
    assert_eq!(delete_response["data"]["deleteTodo"], true);

    // Verify deleted
    let verify_response = execute(
        &pool,
        &format!(r#"query {{ todo(id: "{}") {{ id }} }}"#, todo_id),
    )
    .await;
    assert_no_errors(&verify_response);
    assert!(verify_response["data"]["todo"].is_null());
}
