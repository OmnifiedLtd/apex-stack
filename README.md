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
- **Compile-Time Safe Queries**: SQLx macros (`query!`, `query_as!`) validate your SQL against the database schema at compile time
- **Unified Executor Pattern**: Repositories accept `impl Executor` allowing seamless composition of operations within transactions
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
| **Domain** | Database (SQLx) | Entities, repositories, core business rules |
| **Features** | Domain, Queues | Use cases, orchestration, background jobs |
| **Apps** | Features, HTTP/GraphQL | API exposure, request handling, server lifecycle |

This is a pragmatic take on DDD—the domain layer acknowledges the database as fundamental rather than an implementation detail, while still maintaining clear boundaries.

### Unified Executor Pattern

We use the **Unified Executor Pattern** to share transactions across layers.

```rust
// In Repository (Domain Layer)
pub async fn create<'e, E>(executor: E, ...) -> Result<...>
where E: Executor<'e, Database = Postgres> { ... }

// In Service (Feature Layer)
let mut tx = pool.begin().await?;
UserRepository::create(&mut *tx, ...).await?; // Note: &mut *tx
UserJobs::enqueue_welcome_email(&mut *tx, ...).await?;
tx.commit().await?;
```

This allows atomic composition: if the job enqueue fails, the user creation rolls back.

### Why Not SeaORM?

SeaORM is excellent for many use cases, but it creates friction with sqlxmq's transactional job enqueuing:

```rust
// With SeaORM, getting a raw transaction for sqlxmq is awkward
let txn = db.begin().await?;
let user = user.insert(&txn).await?;
// Now you need to extract the raw SQLx transaction... it's clumsy
```

With SQLx (and our Unified Executor pattern), it's natural and type-safe.

**Trade-offs:**
- SeaORM: Better for complex relations, auto-generated entities, rapid prototyping
- SQLx Macros: Better for performance, compile-time safety, explicit control, and transactional job queues

### Why sqlxmq Over Apalis?

Both are capable job queue libraries, but sqlxmq excels in Postgres-only environments:

| Feature | sqlxmq | Apalis |
|---------|--------|--------|
| Transactional enqueue | First-class | Difficult (separate pool) |
| Dependency weight | Light (macro-based) | Heavy (Tower ecosystem) |
| Backend flexibility | Postgres only | Redis, SQL, Cron, etc. |
| Middleware | Basic retries | Extensive (rate limiting, tracing) |

**The killer feature**: If your user insert commits, the welcome email job commits with it. If anything fails, both roll back. No orphaned jobs, no missing emails.

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

### Offline Mode (CI/CD)

We use `sqlx::query!` macros which require database schema information at compile time. To support CI/CD without a running database, we use SQLx's offline mode.

**If you change any SQL queries:**
1. Ensure your local database is running and migrated.
2. Run: `cargo sqlx prepare --workspace`
3. Commit the updated `.sqlx` directory.

## Testing

The stack is designed for integration testing with real Postgres. Each test gets an isolated, rolled-back transaction or isolated database.

```rust
#[sqlx::test(migrations = "../../migrations")]
async fn test_user_registration_enqueues_job(pool: PgPool) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    
    // Pass transaction to repo
    let user = UserRepository::create(&mut *tx, ...).await?;
    
    // Verify in same transaction or commit and verify
    tx.rollback().await?;
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

The multi-stage Dockerfile creates an optimized production image using SQLx offline mode:

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