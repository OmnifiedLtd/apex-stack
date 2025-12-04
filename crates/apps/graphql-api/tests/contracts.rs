//! GraphQL API contract tests
//!
//! These tests verify that the GraphQL API correctly exposes the feature layer.
//! Focus: mutations/queries exist, return expected types, handle errors properly.
//!
//! Behavior testing happens at the feature layer. These tests ensure the API
//! contract is correct (right fields, right types, right error format).

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

/// Assert response has errors
fn assert_has_errors(response: &Value) {
    let errors = &response["errors"];
    assert!(
        errors.is_array() && !errors.as_array().unwrap().is_empty(),
        "Expected errors but got none"
    );
}

// =============================================================================
// User Mutation Contracts
// =============================================================================

mod user_mutations {
    use super::*;

    #[sqlx::test(migrations = "../../../migrations")]
    async fn register_user_returns_user_type(pool: PgPool) {
        let response = execute(
            &pool,
            r#"
            mutation {
                registerUser(input: { email: "contract@test.com", name: "Contract Test" }) {
                    id
                    email
                    name
                    createdAt
                    updatedAt
                }
            }
            "#,
        )
        .await;

        assert_no_errors(&response);
        let user = &response["data"]["registerUser"];

        // Verify all expected fields are present and have correct types
        assert!(user["id"].is_string(), "id should be a string (UUID)");
        assert!(user["email"].is_string(), "email should be a string");
        assert!(user["name"].is_string(), "name should be a string");
        assert!(
            user["createdAt"].is_string(),
            "createdAt should be a string (DateTime)"
        );
        assert!(
            user["updatedAt"].is_string(),
            "updatedAt should be a string (DateTime)"
        );
    }

    #[sqlx::test(migrations = "../../../migrations")]
    async fn register_user_error_is_graphql_error(pool: PgPool) {
        // First registration
        execute(
            &pool,
            r#"mutation { registerUser(input: { email: "dup@test.com", name: "First" }) { id } }"#,
        )
        .await;

        // Duplicate should return GraphQL error
        let response = execute(
            &pool,
            r#"mutation { registerUser(input: { email: "dup@test.com", name: "Second" }) { id } }"#,
        )
        .await;

        assert_has_errors(&response);
        // Errors should be an array with at least one error object
        let errors = response["errors"].as_array().unwrap();
        assert!(!errors.is_empty());
        // Each error should have a message field
        assert!(errors[0]["message"].is_string());
    }

    #[sqlx::test(migrations = "../../../migrations")]
    async fn update_user_returns_user_type(pool: PgPool) {
        // Create user first
        let create_response = execute(
            &pool,
            r#"mutation { registerUser(input: { email: "update@test.com", name: "Before" }) { id } }"#,
        )
        .await;
        let user_id = create_response["data"]["registerUser"]["id"]
            .as_str()
            .unwrap();

        // Update
        let response = execute(
            &pool,
            &format!(
                r#"mutation {{ updateUser(id: "{}", input: {{ name: "After" }}) {{ id email name }} }}"#,
                user_id
            ),
        )
        .await;

        assert_no_errors(&response);
        let user = &response["data"]["updateUser"];
        assert!(user["id"].is_string());
        assert!(user["email"].is_string());
        assert!(user["name"].is_string());
    }

    #[sqlx::test(migrations = "../../../migrations")]
    async fn delete_user_returns_boolean(pool: PgPool) {
        // Create user
        let create_response = execute(
            &pool,
            r#"mutation { registerUser(input: { email: "delete@test.com", name: "Delete" }) { id } }"#,
        )
        .await;
        let user_id = create_response["data"]["registerUser"]["id"]
            .as_str()
            .unwrap();

        // Delete
        let response = execute(
            &pool,
            &format!(r#"mutation {{ deleteUser(id: "{}") }}"#, user_id),
        )
        .await;

        assert_no_errors(&response);
        assert!(
            response["data"]["deleteUser"].is_boolean(),
            "deleteUser should return boolean"
        );
    }
}

// =============================================================================
// User Query Contracts
// =============================================================================

mod user_queries {
    use super::*;

    #[sqlx::test(migrations = "../../../migrations")]
    async fn user_query_returns_user_or_null(pool: PgPool) {
        // Query non-existent user returns null (not error)
        let response = execute(
            &pool,
            r#"query { user(id: "00000000-0000-0000-0000-000000000000") { id } }"#,
        )
        .await;

        assert_no_errors(&response);
        assert!(
            response["data"]["user"].is_null(),
            "Missing user should return null"
        );
    }

    #[sqlx::test(migrations = "../../../migrations")]
    async fn users_query_returns_array(pool: PgPool) {
        let response = execute(&pool, r#"query { users { id email name } }"#).await;

        assert_no_errors(&response);
        assert!(
            response["data"]["users"].is_array(),
            "users should return an array"
        );
    }

    #[sqlx::test(migrations = "../../../migrations")]
    async fn user_by_email_returns_user_or_null(pool: PgPool) {
        let response = execute(
            &pool,
            r#"query { userByEmail(email: "nonexistent@test.com") { id } }"#,
        )
        .await;

        assert_no_errors(&response);
        assert!(
            response["data"]["userByEmail"].is_null(),
            "Missing user should return null"
        );
    }
}

// =============================================================================
// Todo Mutation Contracts
// =============================================================================

mod todo_mutations {
    use super::*;

    #[sqlx::test(migrations = "../../../migrations")]
    async fn create_todo_returns_todo_type(pool: PgPool) {
        // Create user first
        let user_response = execute(
            &pool,
            r#"mutation { registerUser(input: { email: "todo@test.com", name: "Todo User" }) { id } }"#,
        )
        .await;
        let user_id = user_response["data"]["registerUser"]["id"]
            .as_str()
            .unwrap();

        // Create todo
        let response = execute(
            &pool,
            &format!(
                r#"mutation {{
                    createTodo(input: {{ userId: "{}", title: "Contract Todo", description: "Test" }}) {{
                        id
                        title
                        description
                        status
                        userId
                        createdAt
                        updatedAt
                    }}
                }}"#,
                user_id
            ),
        )
        .await;

        assert_no_errors(&response);
        let todo = &response["data"]["createTodo"];

        assert!(todo["id"].is_string(), "id should be a string");
        assert!(todo["title"].is_string(), "title should be a string");
        assert!(
            todo["description"].is_string() || todo["description"].is_null(),
            "description should be string or null"
        );
        assert!(todo["status"].is_string(), "status should be an enum string");
        assert!(todo["userId"].is_string(), "userId should be a string");
        assert!(todo["createdAt"].is_string(), "createdAt should be a string");
        assert!(todo["updatedAt"].is_string(), "updatedAt should be a string");
    }

    #[sqlx::test(migrations = "../../../migrations")]
    async fn create_todo_for_invalid_user_returns_error(pool: PgPool) {
        let response = execute(
            &pool,
            r#"mutation {
                createTodo(input: { userId: "00000000-0000-0000-0000-000000000000", title: "Orphan" }) {
                    id
                }
            }"#,
        )
        .await;

        assert_has_errors(&response);
    }

    #[sqlx::test(migrations = "../../../migrations")]
    async fn update_todo_returns_todo_type(pool: PgPool) {
        // Setup: create user and todo
        let user_response = execute(
            &pool,
            r#"mutation { registerUser(input: { email: "update-todo@test.com", name: "User" }) { id } }"#,
        )
        .await;
        let user_id = user_response["data"]["registerUser"]["id"]
            .as_str()
            .unwrap();

        let todo_response = execute(
            &pool,
            &format!(
                r#"mutation {{ createTodo(input: {{ userId: "{}", title: "Before" }}) {{ id }} }}"#,
                user_id
            ),
        )
        .await;
        let todo_id = todo_response["data"]["createTodo"]["id"].as_str().unwrap();

        // Update todo
        let response = execute(
            &pool,
            &format!(
                r#"mutation {{ updateTodo(id: "{}", input: {{ title: "After" }}) {{ id title }} }}"#,
                todo_id
            ),
        )
        .await;

        assert_no_errors(&response);
        assert!(response["data"]["updateTodo"]["id"].is_string());
        assert!(response["data"]["updateTodo"]["title"].is_string());
    }

    #[sqlx::test(migrations = "../../../migrations")]
    async fn status_mutations_return_todo_type(pool: PgPool) {
        // Setup
        let user_response = execute(
            &pool,
            r#"mutation { registerUser(input: { email: "status@test.com", name: "User" }) { id } }"#,
        )
        .await;
        let user_id = user_response["data"]["registerUser"]["id"]
            .as_str()
            .unwrap();

        let todo_response = execute(
            &pool,
            &format!(
                r#"mutation {{ createTodo(input: {{ userId: "{}", title: "Status Test" }}) {{ id }} }}"#,
                user_id
            ),
        )
        .await;
        let todo_id = todo_response["data"]["createTodo"]["id"].as_str().unwrap();

        // startTodo returns Todo
        let start_response = execute(
            &pool,
            &format!(
                r#"mutation {{ startTodo(id: "{}") {{ id status }} }}"#,
                todo_id
            ),
        )
        .await;
        assert_no_errors(&start_response);
        assert!(start_response["data"]["startTodo"]["status"].is_string());

        // completeTodo returns Todo
        let complete_response = execute(
            &pool,
            &format!(
                r#"mutation {{ completeTodo(id: "{}") {{ id status }} }}"#,
                todo_id
            ),
        )
        .await;
        assert_no_errors(&complete_response);
        assert!(complete_response["data"]["completeTodo"]["status"].is_string());
    }

    #[sqlx::test(migrations = "../../../migrations")]
    async fn delete_todo_returns_boolean(pool: PgPool) {
        // Setup
        let user_response = execute(
            &pool,
            r#"mutation { registerUser(input: { email: "del-todo@test.com", name: "User" }) { id } }"#,
        )
        .await;
        let user_id = user_response["data"]["registerUser"]["id"]
            .as_str()
            .unwrap();

        let todo_response = execute(
            &pool,
            &format!(
                r#"mutation {{ createTodo(input: {{ userId: "{}", title: "Delete Me" }}) {{ id }} }}"#,
                user_id
            ),
        )
        .await;
        let todo_id = todo_response["data"]["createTodo"]["id"].as_str().unwrap();

        // Delete
        let response = execute(
            &pool,
            &format!(r#"mutation {{ deleteTodo(id: "{}") }}"#, todo_id),
        )
        .await;

        assert_no_errors(&response);
        assert!(response["data"]["deleteTodo"].is_boolean());
    }
}

// =============================================================================
// Todo Query Contracts
// =============================================================================

mod todo_queries {
    use super::*;

    #[sqlx::test(migrations = "../../../migrations")]
    async fn todo_query_returns_todo_or_null(pool: PgPool) {
        let response = execute(
            &pool,
            r#"query { todo(id: "00000000-0000-0000-0000-000000000000") { id } }"#,
        )
        .await;

        assert_no_errors(&response);
        assert!(response["data"]["todo"].is_null());
    }

    #[sqlx::test(migrations = "../../../migrations")]
    async fn todos_for_user_returns_array(pool: PgPool) {
        // Create user
        let user_response = execute(
            &pool,
            r#"mutation { registerUser(input: { email: "list-todos@test.com", name: "User" }) { id } }"#,
        )
        .await;
        let user_id = user_response["data"]["registerUser"]["id"]
            .as_str()
            .unwrap();

        let response = execute(
            &pool,
            &format!(
                r#"query {{ todosForUser(userId: "{}") {{ id title }} }}"#,
                user_id
            ),
        )
        .await;

        assert_no_errors(&response);
        assert!(response["data"]["todosForUser"].is_array());
    }

    #[sqlx::test(migrations = "../../../migrations")]
    async fn todos_for_user_by_status_returns_array(pool: PgPool) {
        // Create user
        let user_response = execute(
            &pool,
            r#"mutation { registerUser(input: { email: "status-list@test.com", name: "User" }) { id } }"#,
        )
        .await;
        let user_id = user_response["data"]["registerUser"]["id"]
            .as_str()
            .unwrap();

        let response = execute(
            &pool,
            &format!(
                r#"query {{ todosForUserByStatus(userId: "{}", status: PENDING) {{ id }} }}"#,
                user_id
            ),
        )
        .await;

        assert_no_errors(&response);
        assert!(response["data"]["todosForUserByStatus"].is_array());
    }
}
