use async_graphql::{Context, Object, Result};
use sqlx::PgPool;
use uuid::Uuid;

use super::types::{TodoStatusType, TodoType, UserType};

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get a user by ID
    async fn user(&self, ctx: &Context<'_>, id: Uuid) -> Result<Option<UserType>> {
        let pool = ctx.data::<PgPool>()?;
        let user = user_feature::UserService::get(pool, id).await.ok();
        Ok(user.map(Into::into))
    }

    /// Get a user by email
    async fn user_by_email(&self, ctx: &Context<'_>, email: String) -> Result<Option<UserType>> {
        let pool = ctx.data::<PgPool>()?;
        let user = user_feature::UserService::get_by_email(pool, &email).await?;
        Ok(user.map(Into::into))
    }

    /// List all users
    async fn users(&self, ctx: &Context<'_>) -> Result<Vec<UserType>> {
        let pool = ctx.data::<PgPool>()?;
        let users = user_feature::UserService::list(pool).await?;
        Ok(users.into_iter().map(Into::into).collect())
    }

    /// Get a todo by ID
    async fn todo(&self, ctx: &Context<'_>, id: Uuid) -> Result<Option<TodoType>> {
        let pool = ctx.data::<PgPool>()?;
        let todo = todo_feature::TodoService::get(pool, id).await.ok();
        Ok(todo.map(Into::into))
    }

    /// List todos for a user
    async fn todos_for_user(&self, ctx: &Context<'_>, user_id: Uuid) -> Result<Vec<TodoType>> {
        let pool = ctx.data::<PgPool>()?;
        let todos = todo_feature::TodoService::list_for_user(pool, user_id).await?;
        Ok(todos.into_iter().map(Into::into).collect())
    }

    /// List todos for a user filtered by status
    async fn todos_for_user_by_status(
        &self,
        ctx: &Context<'_>,
        user_id: Uuid,
        status: TodoStatusType,
    ) -> Result<Vec<TodoType>> {
        let pool = ctx.data::<PgPool>()?;
        let todos =
            todo_feature::TodoService::list_for_user_by_status(pool, user_id, status.into())
                .await?;
        Ok(todos.into_iter().map(Into::into).collect())
    }
}
