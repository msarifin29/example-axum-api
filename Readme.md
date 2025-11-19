# example-axum-api

Lightweight Axum + SQLx example API. This repo is a small scaffold that shows a minimal, runnable HTTP API built with
Axum, Tokio and SQLx (Postgres). It includes a `dev.toml` config, a tiny user CRUD router and simple integration tests.

This README focuses on getting the project running locally quickly.

## What this project contains

- Axum HTTP handlers and routing (see `src/auth/handler.rs`).
- Postgres connection builder reading `dev.toml` (see `src/config/connection.rs`).
- SQLx for DB access and a `migrations/` folder with SQL files.
- Example tests using `axum_test` and tokio.

## Prerequisites

- Rust toolchain (stable) — install via https://rustup.rs
- Cargo on your PATH (installed with rustup)
- Postgres (local or container). You can use Docker if you prefer.
- (Optional) `sqlx-cli` if you want to use `sqlx` migration helpers: `cargo install sqlx-cli --no-default-features --features postgres`

## Configuration

This project reads configuration from a TOML file (`dev.toml` by default). Example (the repo already contains `dev.toml`):

```toml
  name = "development"
  
  [database]
  host = "localhost"
  port = 5432
  user = {user_name}
  password = {password}
  name = "roger_db"
  max_connection = 10
  min_connection = 5
  acquire_timeout = 5
  idle_timeout = 60

  [tcp]
  ip="127.0.0.1"
  port=3000
  ```

Notes:

- The application uses `Configure::build("dev.toml")` so the file name must match the one passed to `ConnectionBuilder` (the code uses `dev.toml`).
- The `dev.toml` contains both DB and TCP settings. The app will bind to the configured `tcp.ip` / `tcp.port`.
- If you encounter issues with environment variables, you can manually export them using the following commands:
  Unix/Linux/macOS
  ```bash
  export DATABASE_URL=$(awk -F'"' '/url=/ {print $2}' dev.toml)
  ```
  windows
  ```bash
  set DATABASE_URL=$(awk -F'"' '/url=/ {print $2}' dev.toml)
  ```

## Database and migrations

The repo includes SQL files in `migrations/` (e.g. `20251114143622_user.up.sql`) — apply them to your database before running the app.

Quick options to create DB and apply migrations:

- Using psql (manual):

```bash
# create database (adjust user/host/port/name to match dev.toml)
psql -U postgres -h localhost -p 5432 -c "CREATE DATABASE roger_db;"

# run migration SQL file(s)
psql -U postgres -h localhost -p 5432 -d roger_db -f migrations/20251114143622_user.up.sql
```

- Using `sqlx-cli` (optional):

```bash
# set DATABASE_URL so sqlx knows where to connect (this is only for sqlx-cli)
export DATABASE_URL="postgres://{user_name}:{password}@localhost:5432/roger_db"

# create database (if using sqlx-cli and migrations managed by sqlx)
sqlx database create

# (If you had sqlx migrations) sqlx migrate run
```

If you don't want to install `sqlx-cli`, using `psql` or a DB GUI is fine for the small example migrations included here.

## Build & run (local)

1. Ensure Postgres is running and `dev.toml` points to a reachable DB.
2. Apply migrations as shown above.
3. Build and run:

```bash
# build
cargo build

# run (debug)
cargo run
```

The server will listen on the IP:port configured in `dev.toml` (default: `127.0.0.1:3000`).

## Endpoints (implemented)

The project exposes a tiny user CRUD API mounted under `/api/users`:

- POST /api/users — create a new user (form encoded). Example fields: `user_name`, `email`, `password`.
- GET  /api/users — list users. Optional query params: `page`, `user_name`.
- PUT  /api/users — update user password (form encoded). Fields: `user_id`, `password`.
- DELETE /api/users/{user_id} — delete user by id.

Example: create a user with curl (form encoded):

```bash
curl -v -X POST http://127.0.0.1:3000/api/users \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "user_name=michael&email=michael@example.com&password=secret"
```

## Tests

- Unit & integration-like tests are present under `src/` using `tokio::test` and `axum_test`.
- To run tests (they expect `dev.toml` and a reachable DB):

```bash
cargo test
```

If you prefer to run tests without hitting your real DB, consider spinning up a temporary Postgres via Docker and pointing `dev.toml` to it.

## Next steps / roadmap

- Add structured migrations (sqlx or refinery) and a simple migration runner.
- Add environment-specific configs (e.g. `prod.toml`) and secrets management.
- Add logging and graceful shutdown hooks.
- Implement request/response DTO docs (OpenAPI) and add more tests for error cases.

## Troubleshooting

- Connection errors: check `dev.toml` values (host/port/user/password/name).
- Port already in use: change `tcp.port` in `dev.toml`.
- If tests fail due to DB state, re-create DB or run migration SQL files again.

---

If you want, I can also:

- add a `Makefile`/`justfile` with run/test commands,
- add a small Docker Compose file with Postgres for local dev,
- or wire up sqlx migrations and a script to run them automatically.

---

Happy hacking!