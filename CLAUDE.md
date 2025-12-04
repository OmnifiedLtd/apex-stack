# CLAUDE.md - Development Guide for Claude Code

This file provides context for Claude Code when working on this project.

## Project Overview

APEX Stack is a Rust web application template using:

- **Axum** - HTTP framework
- **PostgreSQL** - Database
- **SQLx** - Database driver (compile-time checked queries)
- **SeaQuery** - Type-safe SQL query builder (for complex dynamic queries)
- **sqlxmq** - Transactional job queue backed by Postgres
- **async-graphql** - GraphQL API

## Project Structure

```
crates/
├── domain/           # Entities, repositories (knows DB, not HTTP/queues)
├── features/         # Business logic, services, jobs (knows domain + queues)
│   ├── user-feature/
│   └── todo-feature/
└── apps/
    └── graphql-api/  # HTTP layer (knows features, exposes API)
```

## Development Workflow

### Starting Development

```bash
# Start Postgres (uses port 5433 to avoid conflicts)
docker compose up -d
```

If Docker isn't available (e.g., in a cloud container), use the provided setup script. This script will install Postgres, start it, **and run database migrations**.

```bash
# Install and start PostgreSQL natively (uses default port 5432)
./scripts/setup_cloud_db.sh
```

Then continue with:

```bash
# Run the application
cargo run -p graphql-api
```

### Database Migrations

Migrations are embedded and run automatically on app startup.

**Creating a new migration:**

```bash
sqlx migrate add <name>
# Edit the generated .up.sql and .down.sql files
# Restart the app - migrations apply automatically
```

**Rolling back:**

```bash
sqlx migrate revert
```

### SQLx Compile-Time Checks

This project uses `sqlx::query!()` and `sqlx::query_as!()` macros which check your SQL against the database schema at compile time.

**Offline Mode (CI/Docker):**
To allow compilation without a running database (e.g., in CI), we use SQLx offline mode. The schema data is stored in the `.sqlx/` directory.

**Workflow:**

1. Start the database: `docker compose up -d`
2. Make changes to your code or SQL queries.
3. If you changed any queries, update the offline data:
   ```bash
   export DATABASE_URL=postgres://postgres:postgres@localhost:5433/apex_stack
   cargo sqlx prepare --workspace
   ```
4. Commit the `.sqlx/` directory changes.

**Note:** If you get compilation errors about "database error", make sure your DB is running and you have run migrations, or that your `.sqlx` data is up to date.

### Key Patterns

#### Unified Executor Pattern

Repositories accept `impl sqlx::Executor` to allow seamless reuse of transactions across layers.

```rust
// In Repository
pub async fn create<'e, E>(executor: E, ...) -> Result<...>
where E: Executor<'e, Database = Postgres> { ... }

// In Service (Atomic Workflow)
let mut tx = pool.begin().await?;
UserRepository::create(&mut *tx, ...).await?; // Note the &mut *tx
Job::spawn(&mut *tx).await?;
tx.commit().await?;
```

#### Transactional Job Enqueue (the "killer feature")

User creation and welcome email job are atomic:

```rust
let mut tx = pool.begin().await?;
UserRepository::create(&mut *tx, &email, &name).await?;
UserJobs::enqueue_welcome_email(&mut *tx, user_id, email, name).await?;
tx.commit().await?;  // Both succeed or both fail
```

#### SeaQuery for Dynamic Queries (Optional)

For complex search/filter queries where macros are too rigid, we use SeaQuery:

```rust
let (sql, values) = Query::select()
    .columns([Users::Id, Users::Email])
    .from(Users::Table)
    .and_where(Expr::col(Users::Email).eq(email))
    .build_sqlx(PostgresQueryBuilder);

sqlx::query_as_with::<_, User, _>(&sql, values)
    .fetch_optional(executor)
    .await?
```

### Testing

**CRITICAL: Tests MUST always be run before completing any task and they MUST pass.**

```bash
# Run all tests (requires Postgres running on port 5433)
DATABASE_URL="postgres://postgres:postgres@localhost:5433/apex_stack" cargo test

# Run tests for a specific crate
DATABASE_URL="postgres://postgres:postgres@localhost:5433/apex_stack" cargo test -p domain
DATABASE_URL="postgres://postgres:postgres@localhost:5433/apex_stack" cargo test -p user-feature
DATABASE_URL="postgres://postgres:postgres@localhost:5433/apex_stack" cargo test -p todo-feature
```

**How tests work:**

- Integration tests use `#[sqlx::test]` which creates an isolated database per test
- Migrations run automatically before each test
- Tests are fully isolated - no test pollution
- Requires Postgres running (`docker compose up -d`)

**Test locations:**

- `crates/domain/tests/` - Repository integration tests (User, Todo)
- `crates/features/user-feature/tests/` - BDD behavior tests + user journey tests
- `crates/features/todo-feature/tests/` - BDD behavior tests + todo workflow tests
- `crates/apps/graphql-api/tests/` - API contract tests + smoke tests

### Test Philosophy

**Domain tests:** Verify database operations work correctly.
Focus on CRUD, constraints, and edge cases. Use `tx.rollback()` to keep tests fast/clean if manually managing transactions, though `#[sqlx::test]` handles isolation well too.

**Feature tests:** Verify business behaviors work correctly.
Use BDD-style naming (`user_can_register`, `todo_can_be_completed`).
This is where user journeys and workflows are tested - transport agnostic.

**API tests:** Verify the API layer correctly exposes features.
Contract tests (right fields, right types) and smoke tests (happy paths).
Keep minimal - behaviors are tested at the feature layer.

### Adding a New Feature

1. Create crate: `mkdir -p crates/features/my-feature/src`
2. Add to workspace `Cargo.toml` members
3. Implement:
   - `error.rs` - Feature errors
   - `service.rs` - Business logic
   - `jobs.rs` - Background jobs (optional)
4. Expose in `graphql-api` schema

### sqlxmq Migrations

The `migrations/` folder contains sqlxmq migrations prefixed with `sqlxmq_`. These are copied from the [sqlxmq crate](https://github.com/Diggsey/sqlxmq) with both `.up.sql` and `.down.sql` files for reversibility.

## Common Tasks

| Task               | Command                                                                            |
| ------------------ | ---------------------------------------------------------------------------------- |
| Start Postgres     | `docker compose up -d`                                                             |
| Stop Postgres      | `docker compose down`                                                              |
| Reset database     | `docker compose down -v && docker compose up -d`                                   |
| Run app            | `cargo run -p graphql-api`                                                         |
| Run tests          | `DATABASE_URL="postgres://postgres:postgres@localhost:5433/apex_stack" cargo test` |
| Check compilation  | `cargo check`                                                                      |
| Update SQLx data   | `cargo sqlx prepare --workspace`                                                   |
| Add migration      | `sqlx migrate add <name>`                                                          |
| Rollback migration | `sqlx migrate revert`                                                              |

## Environment Variables

Set in `.env` (copied from `.env.example`):

- `DATABASE_URL` - Postgres connection string (default uses port 5433)
- `LISTEN_ADDR` - Server address (default: `0.0.0.0:3000`)
- `RUST_LOG` - Log levels