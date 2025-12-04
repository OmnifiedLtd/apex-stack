use std::env;

use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::State,
    routing::{get, post},
    Router,
};
use graphql_api::{build_schema, AppSchema};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub schema: AppSchema,
}

/// GraphQL handler
async fn graphql_handler(
    State(state): State<AppState>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    state.schema.execute(req.into_inner()).await.into()
}

/// GraphQL Playground handler
async fn graphql_playground() -> impl axum::response::IntoResponse {
    axum::response::Html(async_graphql::http::playground_source(
        async_graphql::http::GraphQLPlaygroundConfig::new("/graphql"),
    ))
}

/// Health check handler
async fn health() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "graphql_api=debug,user_feature=debug,todo_feature=debug,sqlx=warn".into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Database connection
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/apex_stack".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await?;

    info!("Connected to database");

    // Run migrations
    sqlx::migrate!("../../../migrations")
        .run(&pool)
        .await?;

    info!("Migrations complete");

    // Build GraphQL schema
    let schema = build_schema(pool.clone());

    // Create app state
    let state = AppState {
        pool: pool.clone(),
        schema,
    };

    // Start the job runner for email processing
    let job_pool = pool.clone();
    let email_runner = tokio::spawn(async move {
        let registry = user_feature::UserJobs::registry();

        info!("Starting email job runner");

        let runner = registry
            .runner(&job_pool)
            .set_channel_names(&["emails"])
            .set_concurrency(2, 10)
            .run()
            .await;

        if let Err(e) = runner {
            tracing::error!("Job runner error: {}", e);
        }
    });

    // Build router
    let app = Router::new()
        .route("/graphql", post(graphql_handler))
        .route("/playground", get(graphql_playground))
        .route("/health", get(health))
        .with_state(state);

    // Start server
    let addr = env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("GraphQL Playground: http://{}/playground", addr);
    info!("GraphQL endpoint: http://{}/graphql", addr);

    axum::serve(listener, app).await?;

    email_runner.abort();

    Ok(())
}
