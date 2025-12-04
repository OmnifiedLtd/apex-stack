use domain::{Todo, TodoRepository, TodoStatus, UserRepository};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::TodoFeatureError;

/// Input for creating a new todo
pub struct CreateTodoInput {
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
}

/// Input for updating a todo
pub struct UpdateTodoInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<TodoStatus>,
}

/// Service for todo-related operations
pub struct TodoService;

impl TodoService {
    /// Create a new todo for a user
    pub async fn create(pool: &PgPool, input: CreateTodoInput) -> Result<Todo, TodoFeatureError> {
        // Verify user exists
        if UserRepository::find_by_id(pool, input.user_id)
            .await?
            .is_none()
        {
            return Err(TodoFeatureError::UserNotFound(input.user_id));
        }

        let todo = TodoRepository::create(
            pool,
            input.user_id,
            &input.title,
            input.description.as_deref(),
        )
        .await?;

        Ok(todo)
    }

    /// Get a todo by ID
    pub async fn get(pool: &PgPool, id: Uuid) -> Result<Todo, TodoFeatureError> {
        TodoRepository::find_by_id(pool, id)
            .await?
            .ok_or(TodoFeatureError::NotFound(id))
    }

    /// List todos for a user
    pub async fn list_for_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<Todo>, TodoFeatureError> {
        Ok(TodoRepository::list_by_user(pool, user_id).await?)
    }

    /// List todos for a user filtered by status
    pub async fn list_for_user_by_status(
        pool: &PgPool,
        user_id: Uuid,
        status: TodoStatus,
    ) -> Result<Vec<Todo>, TodoFeatureError> {
        Ok(TodoRepository::list_by_user_and_status(pool, user_id, status).await?)
    }

    /// Update a todo
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        input: UpdateTodoInput,
    ) -> Result<Todo, TodoFeatureError> {
        // Get existing todo to merge updates
        let existing = Self::get(pool, id).await?;

        // Update status if provided
        if let Some(status) = input.status {
            if status != existing.status {
                return TodoRepository::update_status(pool, id, status)
                    .await?
                    .ok_or(TodoFeatureError::NotFound(id));
            }
        }

        // Update content if title or description changed
        if input.title.is_some() || input.description.is_some() {
            let new_title = input.title.as_deref().unwrap_or(&existing.title);
            let new_description = input
                .description
                .as_deref()
                .or(existing.description.as_deref());

            return TodoRepository::update_content(pool, id, new_title, new_description)
                .await?
                .ok_or(TodoFeatureError::NotFound(id));
        }

        Ok(existing)
    }

    /// Mark a todo as completed
    pub async fn complete(pool: &PgPool, id: Uuid) -> Result<Todo, TodoFeatureError> {
        TodoRepository::update_status(pool, id, TodoStatus::Completed)
            .await?
            .ok_or(TodoFeatureError::NotFound(id))
    }

    /// Mark a todo as in progress
    pub async fn start(pool: &PgPool, id: Uuid) -> Result<Todo, TodoFeatureError> {
        TodoRepository::update_status(pool, id, TodoStatus::InProgress)
            .await?
            .ok_or(TodoFeatureError::NotFound(id))
    }

    /// Delete a todo
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, TodoFeatureError> {
        Ok(TodoRepository::delete(pool, id).await?)
    }
}
