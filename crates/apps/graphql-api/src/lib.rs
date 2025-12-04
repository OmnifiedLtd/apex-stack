pub mod schema;

use async_graphql::{EmptySubscription, Schema};
use schema::{MutationRoot, QueryRoot};
use sqlx::PgPool;

/// The GraphQL schema type
pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

/// Build the GraphQL schema with the given database pool
pub fn build_schema(pool: PgPool) -> AppSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(pool)
        .finish()
}
