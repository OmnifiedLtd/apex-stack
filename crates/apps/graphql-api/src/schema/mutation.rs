use async_graphql::{Context, Object, Result};
use sqlx::PgPool;
use uuid::Uuid;

use super::types::{CreateTodoInput, CreateUserInput, TodoType, UpdateTodoInput, UpdateUserInput, UserType};

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Register a new user (sends welcome email)
    async fn register_user(&self, ctx: &Context<'_>, input: CreateUserInput) -> Result<UserType> {
        let pool = ctx.data::<PgPool>()?;
        let user = user_feature::UserService::register(
            pool,
            user_feature::CreateUserInput {
                email: input.email,
                name: input.name,
            },
        )
        .await?;
        Ok(user.into())
    }

    /// Update a user
    async fn update_user(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
        input: UpdateUserInput,
    ) -> Result<UserType> {
        let pool = ctx.data::<PgPool>()?;
        let user = user_feature::UserService::update(
            pool,
            id,
            user_feature::UpdateUserInput { name: input.name },
        )
        .await?;
        Ok(user.into())
    }

    /// Delete a user
    async fn delete_user(&self, ctx: &Context<'_>, id: Uuid) -> Result<bool> {
        let pool = ctx.data::<PgPool>()?;
        Ok(user_feature::UserService::delete(pool, id).await?)
    }

    /// Create a new todo
    async fn create_todo(&self, ctx: &Context<'_>, input: CreateTodoInput) -> Result<TodoType> {
        let pool = ctx.data::<PgPool>()?;
        let todo = todo_feature::TodoService::create(
            pool,
            todo_feature::CreateTodoInput {
                user_id: input.user_id,
                title: input.title,
                description: input.description,
            },
        )
        .await?;
        Ok(todo.into())
    }

    /// Update a todo
    async fn update_todo(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
        input: UpdateTodoInput,
    ) -> Result<TodoType> {
        let pool = ctx.data::<PgPool>()?;
        let todo = todo_feature::TodoService::update(
            pool,
            id,
            todo_feature::UpdateTodoInput {
                title: input.title,
                description: input.description,
                status: input.status.map(Into::into),
            },
        )
        .await?;
        Ok(todo.into())
    }

    /// Mark a todo as completed
    async fn complete_todo(&self, ctx: &Context<'_>, id: Uuid) -> Result<TodoType> {
        let pool = ctx.data::<PgPool>()?;
        let todo = todo_feature::TodoService::complete(pool, id).await?;
        Ok(todo.into())
    }

    /// Mark a todo as in progress
    async fn start_todo(&self, ctx: &Context<'_>, id: Uuid) -> Result<TodoType> {
        let pool = ctx.data::<PgPool>()?;
        let todo = todo_feature::TodoService::start(pool, id).await?;
        Ok(todo.into())
    }

    /// Delete a todo
    async fn delete_todo(&self, ctx: &Context<'_>, id: Uuid) -> Result<bool> {
        let pool = ctx.data::<PgPool>()?;
        Ok(todo_feature::TodoService::delete(pool, id).await?)
    }
}
