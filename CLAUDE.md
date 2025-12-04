# CLAUDE.md - Development Guide for Claude Code

This file provides context for Claude Code when working on this project.

## Project Overview

APEX Stack is a Rust web application template using:

- **Axum** - HTTP framework
- **PostgreSQL** - Database
- **SQLx** - Database driver (runtime queries via SeaQuery, not compile-time macros)
- **SeaQuery** - Type-safe SQL query builder
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

# Run the application (migrations run automatically)
cargo run -p graphql-api
```

### Database Migrations

Migrations are embedded and run automatically on app startup. No manual steps needed.

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

### Key Patterns

#### Transactional Job Enqueue (the "killer feature")

User creation and welcome email job are atomic:

```rust
let mut tx = pool.begin().await?;
UserRepository::create(&mut tx, &email, &name).await?;
UserJobs::enqueue_welcome_email(&mut tx, user_id, email, name).await?;
tx.commit().await?;  // Both succeed or both fail
```

#### SeaQuery for Dynamic Queries

```rust
let (sql, values) = Query::select()
    .columns([Users::Id, Users::Email])
    .from(Users::Table)
    .and_where(Expr::col(Users::Email).eq(email))
    .build_sqlx(PostgresQueryBuilder);

sqlx::query_as_with::<_, User, _>(&sql, values)
    .fetch_optional(pool)
    .await?
```

#### No `cargo sqlx prepare` Needed

This stack uses SeaQuery (runtime query building) instead of `sqlx::query!()` macros (compile-time checked). Benefits:

- No `.sqlx/` cache to maintain
- Docker/CI builds don't need a running database
- Trade-off: Query errors are runtime, not compile-time

### Testing

```bash
# Run tests (requires Postgres running)
cargo test
```

Integration tests use `#[sqlx::test]` which creates isolated databases per test.

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

| Task               | Command                                          |
| ------------------ | ------------------------------------------------ |
| Start Postgres     | `docker compose up -d`                           |
| Stop Postgres      | `docker compose down`                            |
| Reset database     | `docker compose down -v && docker compose up -d` |
| Run app            | `cargo run -p graphql-api`                       |
| Run tests          | `cargo test`                                     |
| Check compilation  | `cargo check`                                    |
| Add migration      | `sqlx migrate add <name>`                        |
| Rollback migration | `sqlx migrate revert`                            |

## Environment Variables

Set in `.env` (copied from `.env.example`):

- `DATABASE_URL` - Postgres connection string (default uses port 5433)
- `LISTEN_ADDR` - Server address (default: `0.0.0.0:3000`)
- `RUST_LOG` - Log levels
