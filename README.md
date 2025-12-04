# APEX Stack

A modern, high-performance Rust web application stack built on **A**xum, **P**ostgres, and SQL**X**.

## Quickstart

### Prerequisites

- Rust 1.85+ (edition 2024)
- PostgreSQL 14+
- Docker (optional, for local Postgres)

### Setup

```bash
# Clone and enter the project
cd apex-stack

# Start Postgres
docker compose up -d

# Configure environment
cp .env.example .env

# Run the application
cargo run -p graphql-api
```

### Usage

Open the GraphQL Playground at `http://localhost:3000/playground` and try:

```graphql
# Register a user (triggers welcome email job)
mutation {
  registerUser(input: { email: "alice@example.com", name: "Alice" }) {
    id
    email
    name
  }
}

# Create a todo
mutation {
  createTodo(input: {
    userId: "uuid-from-above",
    title: "Learn APEX Stack",
    description: "Build something awesome"
  }) {
    id
    title
    status
  }
}

# Query users and their todos
query {
  users {
    id
    name
    email
  }
  todosForUser(userId: "uuid-from-above") {
    id
    title
    status
  }
}
```

## Key Features

- **Transactional Atomicity**: Database writes and job enqueues happen in a single transaction—if one fails, both roll back
- **Type-Safe Query Building**: SeaQuery provides compile-time safe dynamic queries without ORM overhead
- **Postgres-Native Job Queue**: sqlxmq uses `FOR UPDATE SKIP LOCKED` for high-performance, reliable background jobs
- **GraphQL API**: async-graphql with Axum for a modern, type-safe API layer
- **Layered Architecture**: Clean separation between domain, features, and application layers

## Project Structure

```
apex-stack/
├── Cargo.toml                    # Workspace configuration
├── migrations/                   # SQL migrations (sqlx-cli)
│   ├── *_create_users.sql
│   ├── *_create_todos.sql
│   └── *_create_jobs.sql
└── crates/
    ├── domain/                   # Domain layer
    │   └── src/
    │       ├── user.rs           # User entity & repository
    │       ├── todo.rs           # Todo entity & repository
    │       └── error.rs          # Domain errors
    ├── features/                 # Feature modules
    │   ├── user-feature/
    │   │   └── src/
    │   │       ├── service.rs    # User registration (atomic)
    │   │       └── jobs.rs       # Welcome email job
    │   └── todo-feature/
    │       └── src/
    │           └── service.rs    # Todo CRUD operations
    └── apps/
        └── graphql-api/          # Application layer
            └── src/
                ├── main.rs       # Server & job runner
                └── schema/       # GraphQL schema
```

## Architecture

### Layer Responsibilities

| Layer | Knows About | Responsibility |
|-------|-------------|----------------|
| **Domain** | Database (SQLx, SeaQuery) | Entities, repositories, core business rules |
| **Features** | Domain, Queues | Use cases, orchestration, background jobs |
| **Apps** | Features, HTTP/GraphQL | API exposure, request handling, server lifecycle |

This is a pragmatic take on DDD—the domain layer acknowledges the database as fundamental rather than an implementation detail, while still maintaining clear boundaries.

### Why Not SeaORM?

SeaORM is excellent for many use cases, but it creates friction with sqlxmq's transactional job enqueuing:

```rust
// With SeaORM, getting a raw transaction for sqlxmq is awkward
let txn = db.begin().await?;
let user = user.insert(&txn).await?;
// Now you need to extract the raw SQLx transaction... it's clumsy
```

```rust
// With SQLx + SeaQuery, it's natural
let mut tx = pool.begin().await?;
UserRepository::create(&mut tx, &email, &name).await?;
UserJobs::enqueue_welcome_email(&mut tx, user_id, email, name).await?;
tx.commit().await?;  // Both succeed or both fail
```

**Trade-offs:**
- SeaORM: Better for complex relations, auto-generated entities, rapid prototyping
- SQLx + SeaQuery: Better for performance, compile times, transactional job queues

### Why sqlxmq Over Apalis?

Both are capable job queue libraries, but sqlxmq excels in Postgres-only environments:

| Feature | sqlxmq | Apalis |
|---------|--------|--------|
| Transactional enqueue | First-class | Difficult (separate pool) |
| Dependency weight | Light (macro-based) | Heavy (Tower ecosystem) |
| Backend flexibility | Postgres only | Redis, SQL, Cron, etc. |
| Middleware | Basic retries | Extensive (rate limiting, tracing) |

**The killer feature**: If your user insert commits, the welcome email job commits with it. If anything fails, both roll back. No orphaned jobs, no missing emails.

### Why SeaQuery?

SeaQuery sits between raw SQL strings and a full ORM:

```rust
// Dynamic query building without string concatenation
let mut query = Query::select();
query.columns([Users::Id, Users::Email]).from(Users::Table);

if let Some(email) = email_filter {
    query.and_where(Expr::col(Users::Email).eq(email));
}

if let Some(active) = is_active {
    query.and_where(Expr::col(Users::Active).eq(active));
}

let (sql, values) = query.build_sqlx(PostgresQueryBuilder);
```

Benefits:
- **Type-safe column names** via `Iden` derive macro
- **Automatic parameterization** prevents SQL injection
- **No runtime overhead** of an ORM's query generation
- **Composable** for complex dynamic queries

#### Trade-off: SeaQuery vs SQLx `query!` Macros

SQLx's `query!()` macros offer **compile-time SQL validation**—queries are checked against your actual database schema during compilation. This is powerful for large teams where schema refactoring (renaming columns, changing types) could break queries scattered across the codebase. With `query!` macros, a renamed column surfaces as compile errors everywhere it's used.

SeaQuery validates queries at **runtime instead of compile-time**. We chose this trade-off because:
1. **sqlxmq integration** - Transactional job enqueuing requires raw `Transaction` objects, which work more naturally with SeaQuery than `query!` macros
2. **Simpler CI/CD** - No need to maintain a `.sqlx/` cache or have a database available during builds
3. **Dynamic queries** - Conditional WHERE clauses and filters are more ergonomic

**Mitigating the runtime validation trade-off:**
- **Avoid breaking schema changes** - Prefer additive migrations (new columns, new tables) over destructive ones (renaming columns, changing types). When you must rename, add the new column first, migrate data, then remove the old one in a later release
- **Test your queries** - Write integration tests that exercise your repository methods. The `#[sqlx::test]` attribute makes this easy by providing isolated test databases (see Testing section below)

If you're not using transactional job queues and prefer compile-time safety, consider using `sqlx::query!()` macros instead of SeaQuery.

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `postgres://postgres:postgres@localhost/apex_stack` | PostgreSQL connection string |
| `LISTEN_ADDR` | `0.0.0.0:3000` | Server bind address |
| `RUST_LOG` | `graphql_api=debug` | Log level configuration |

## Database Migrations

Migrations run automatically on application startup via `sqlx::migrate!()`. No manual migration step is needed during development.

### Creating New Migrations

```bash
# Install sqlx-cli (one-time setup)
cargo install sqlx-cli --no-default-features --features native-tls,postgres

# Create a new migration (generates timestamped .up.sql and .down.sql files)
sqlx migrate add create_something

# Manual rollback if needed
sqlx migrate revert
```

### Why No `cargo sqlx prepare`?

This stack uses **SeaQuery** for dynamic query building instead of `sqlx::query!()` macros. This means:

- **No offline compilation cache needed** - queries are built at runtime, not compile-time
- **Simpler CI/Docker builds** - no need for a database connection during compilation
- **Trade-off**: No compile-time query validation (runtime errors instead)

The `sqlx::migrate!()` macro embeds migration files at compile-time but doesn't require database access.

## Testing

The stack is designed for integration testing with real Postgres:

```rust
#[sqlx::test(migrations = "./migrations")]
async fn test_user_registration_enqueues_job(pool: PgPool) -> sqlx::Result<()> {
    // Each test gets an isolated database
    let user = UserService::register(&pool, CreateUserInput {
        email: "test@example.com".into(),
        name: "Test".into(),
    }).await?;

    // Verify job was enqueued atomically
    let job_count = sqlx::query!("SELECT count(*) as c FROM mq_msgs")
        .fetch_one(&pool)
        .await?.c.unwrap();

    assert_eq!(job_count, 1);
    Ok(())
}
```

## Adding a New Feature

1. Create the feature crate:
   ```bash
   mkdir -p crates/features/my-feature/src
   ```

2. Add to workspace `Cargo.toml`:
   ```toml
   members = [
       # ...
       "crates/features/my-feature",
   ]
   ```

3. Implement the feature following the pattern:
   - `error.rs` - Feature-specific errors
   - `service.rs` - Business logic and orchestration
   - `jobs.rs` - Background jobs (if needed)

4. Expose in the app layer (add to GraphQL schema, REST routes, etc.)

## Docker

This project includes two Docker configurations for different purposes:

### Local Development (`docker-compose.yml`)

For local development, we run Postgres in Docker while running the Rust application directly on the host machine. This approach provides:

- **Fast iteration** - No container rebuilds on code changes
- **Native compilation speed** - Rust compiles faster outside containers
- **Full tooling access** - rust-analyzer, cargo-watch, debuggers work seamlessly

```bash
# Start Postgres
docker compose up -d

# Run the app on your host
cargo run -p graphql-api

# Or with auto-reload
cargo watch -x 'run -p graphql-api'
```

### Production Deployment (`Dockerfile`)

The multi-stage Dockerfile creates an optimized production image:

- **Stage 1 (Builder)**: Compiles the release binary with all dependencies
- **Stage 2 (Runtime)**: Minimal Debian image with just the binary (~50MB)

```bash
# Build the production image
docker build -t apex-stack .

# Run it (requires DATABASE_URL)
docker run -e DATABASE_URL=postgres://... -p 3000:3000 apex-stack
```

#### Deploying to Railway

```bash
# Railway will auto-detect the Dockerfile
railway up
```

Set the `DATABASE_URL` environment variable in Railway to your Postgres connection string.

#### Deploying to Fly.io

```bash
# Initialize (first time only)
fly launch

# Deploy
fly deploy
```

## License

MIT
