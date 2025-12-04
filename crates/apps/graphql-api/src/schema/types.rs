use async_graphql::{Enum, InputObject, SimpleObject};
use time::OffsetDateTime;
use uuid::Uuid;

/// GraphQL representation of a User
#[derive(SimpleObject)]
pub struct UserType {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<domain::User> for UserType {
    fn from(user: domain::User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

/// GraphQL representation of a Todo
#[derive(SimpleObject)]
pub struct TodoType {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TodoStatusType,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl From<domain::Todo> for TodoType {
    fn from(todo: domain::Todo) -> Self {
        Self {
            id: todo.id,
            user_id: todo.user_id,
            title: todo.title,
            description: todo.description,
            status: todo.status.into(),
            created_at: todo.created_at,
            updated_at: todo.updated_at,
        }
    }
}

/// GraphQL enum for Todo status
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum TodoStatusType {
    Pending,
    InProgress,
    Completed,
}

impl From<domain::TodoStatus> for TodoStatusType {
    fn from(status: domain::TodoStatus) -> Self {
        match status {
            domain::TodoStatus::Pending => TodoStatusType::Pending,
            domain::TodoStatus::InProgress => TodoStatusType::InProgress,
            domain::TodoStatus::Completed => TodoStatusType::Completed,
        }
    }
}

impl From<TodoStatusType> for domain::TodoStatus {
    fn from(status: TodoStatusType) -> Self {
        match status {
            TodoStatusType::Pending => domain::TodoStatus::Pending,
            TodoStatusType::InProgress => domain::TodoStatus::InProgress,
            TodoStatusType::Completed => domain::TodoStatus::Completed,
        }
    }
}

/// Input for creating a user
#[derive(InputObject)]
pub struct CreateUserInput {
    pub email: String,
    pub name: String,
}

/// Input for updating a user
#[derive(InputObject)]
pub struct UpdateUserInput {
    pub name: Option<String>,
}

/// Input for creating a todo
#[derive(InputObject)]
pub struct CreateTodoInput {
    pub user_id: Uuid,
    pub title: String,
    pub description: Option<String>,
}

/// Input for updating a todo
#[derive(InputObject)]
pub struct UpdateTodoInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<TodoStatusType>,
}
