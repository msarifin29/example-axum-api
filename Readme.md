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

## WebSocket Endpoint

The project also includes a **real-time WebSocket** connection handler. See `src/websocket/handler.rs` for the implementation.

**WebSocket Route:** `ws://localhost:3000/ws?user_id={user_id}`

### How it works

1. **Connect** with a valid `user_id` query parameter.
2. **Server validates** the user exists in the database.
3. If valid: connection upgrades, server sends welcome message.
4. If invalid: server returns `401 Unauthorized`.
5. **Message exchange**:
   - Client sends **text** → Server echoes back with user data in JSON
   - Client sends **binary** → Server responds with pong frame
   - Client sends **ping** → Server responds with pong (keep-alive)
   - Client sends **close** → Connection terminates gracefully

### Example: Connect with wscat

First, create a user via the REST API:

```bash
curl -X POST http://127.0.0.1:3000/api/users \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "user_name=alice&email=alice@example.com&password=secret123"
```

Note the returned `user_id` from the response. Then connect to WebSocket:

```bash
# Install websocat
cargo install websocat

websocat ws://localhost:3000/ws?user_id=YOUR_USER_ID
```

### Message Types Handled

| Client Sends | Server Responds |
|---|---|
| **Text** | JSON with user data + echoed message |
| **Binary** | Pong frame (keep-alive) |
| **Ping** | Pong frame (keep-alive) |
| **Close** | Closes connection gracefully |

For detailed implementation, see:
- `src/websocket/handler.rs` — WebSocket handler logic with full documentation
- `src/websocket/mod.rs` — Route registration

## Private Chat (End-to-End)

The project includes a **private messaging feature** for real-time chat between two users. See `src/websocket/chat.rs` for the implementation.

**Private Chat Route:** `ws://localhost:3000/ws/chat?sender_id={sender_id}&receiver_id={receiver_id}`

### How Private Chat Works

1. **Two users connect** with their `sender_id` and `receiver_id` query parameters.
2. **Both users must exist** in the database (server validates both user_ids).
3. **Broadcast channels** route messages between connected users in real-time.
4. **Message format**: JSON with sender_id, receiver_id, message, and timestamp.
5. **Connection tracking**: Each active connection is registered in a HashMap for efficient routing.

### End-to-End Chat Testing

#### Step 1: Create two test users

```bash
# Create first user (Michael)
USER1=$(curl -s -X POST http://127.0.0.1:3000/api/users \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "user_name=michael&email=michael@example.com&password=pass123" | jq -r '.data.user_id')

echo "Michael user_id: $USER1"

# Create second user (Marley)
USER2=$(curl -s -X POST http://127.0.0.1:3000/api/users \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "user_name=marley&email=marley@example.com&password=pass123" | jq -r '.data.user_id')

echo "Marley user_id: $USER2"
```

#### Step 2: Open two terminal windows for testing

**Terminal 1 (Alice connects to chat):**

```bash
websocat "ws://localhost:3000/ws/chat?sender_id=$USER1&receiver_id=$USER2"
```

**Terminal 2 (Bob connects to chat):**

```bash
websocat "ws://localhost:3000/ws/chat?sender_id=$USER2&receiver_id=$USER1"
```

#### Step 3: Send messages

In **Terminal 1**, type a message (Alice sends to Bob):
```
> Hello Bob, how are you?
```

You'll see in **Terminal 2** (Bob receives):
```
< {"sender_id":"<USER1>","receiver_id":"<USER2>","message":"Hello Bob, how are you?","timestamp":1700XXX}
```

In **Terminal 2**, Bob replies (Bob sends to Alice):
```
> Hi Alice! I'm doing great!
```

You'll see in **Terminal 1** (Alice receives):
```
< {"sender_id":"<USER2>","receiver_id":"<USER1>","message":"Hi Alice! I'm doing great!","timestamp":1700XXX}
```

### Private Chat Implementation Details

- **Location**: `src/websocket/chat.rs`
- **State Management**: `PrivateChatState` tracks active connections per user using broadcast channels
- **Routing**: `send_to_user()` looks up receiver in active connections and broadcasts message
- **Error Handling**: Returns JSON error if message serialization fails
- **Connection Cleanup**: Removes user from connections map on disconnect

For implementation details and documentation, see `src/websocket/chat.rs`.

## Group Chat

The project includes a **multi-user group messaging feature** for real-time conversations among multiple users in a group. See `src/websocket/group.rs` for the WebSocket implementation and `src/group/handler.rs` for group management.

**Group Chat Route:** `ws://localhost:3000/ws/group?user_id={user_id}&group_id={group_id}`

### How Group Chat Works

1. **User joins a group** with valid `user_id` and `group_id` query parameters.
2. **Server validates** both the user and group exist in the database.
3. **Broadcast channel** delivers all messages to ALL connected group members.
4. **Message format**: JSON with user_name, message, and optional error handling.
5. **Welcome notification**: When a user joins, a welcome message is broadcast to all members.
6. **Scalable**: Single broadcast channel handles multiple concurrent connections efficiently.

### Group Management API

Before joining a group chat, create a group via REST API.

#### Create a Group

```bash
curl -X POST http://127.0.0.1:3000/api/groups \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "name=Tech_Enthusiasts&description=A group for tech discussions"
```

#### List All Groups

```bash
curl http://127.0.0.1:3000/api/groups/1
```

Returns paginated groups (10 per page, page 1).

### End-to-End Group Chat Testing

#### Step 1: Create users and a group

```bash
# Create users
USER1=$(curl -s -X POST http://127.0.0.1:3000/api/users \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "user_name=alice&email=alice@example.com&password=pass123" | jq -r '.data.user_id')

USER2=$(curl -s -X POST http://127.0.0.1:3000/api/users \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "user_name=bob&email=bob@example.com&password=pass123" | jq -r '.data.user_id')

USER3=$(curl -s -X POST http://127.0.0.1:3000/api/users \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "user_name=charlie&email=charlie@example.com&password=pass123" | jq -r '.data.user_id')

# Create group
GROUP=$(curl -s -X POST http://127.0.0.1:3000/api/groups \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "name=DevChat&description=Developers chatting" | jq -r '.data.group_id')

echo "Users: $USER1, $USER2, $USER3"
echo "Group: $GROUP"
```

#### Step 2: Open three terminal windows for testing

**Terminal 1 (Alice joins group):**
```bash
websocat "ws://localhost:3000/ws/group?user_id=$USER1&group_id=$GROUP"
```

**Terminal 2 (Bob joins group):**
```bash
websocat "ws://localhost:3000/ws/group?user_id=$USER2&group_id=$GROUP"
```

**Terminal 3 (Charlie joins group):**
```bash
websocat "ws://localhost:3000/ws/group?user_id=$USER3&group_id=$GROUP"
```

#### Step 3: Broadcast messages to all members

In **Terminal 1** (Alice sends):
```
> Hello everyone! Welcome to the group chat!
```

All three terminals receive:
```
< {"user_name":"alice","message":"Hello everyone! Welcome to the group chat!"}
```

In **Terminal 2** (Bob replies):
```
> Hey Alice! Great to have a group chat feature!
```

All three terminals receive:
```
< {"user_name":"bob","message":"Hey Alice! Great to have a group chat feature!"}
```

In **Terminal 3** (Charlie joins in):
```
> This is awesome! Love the real-time updates!
```

All three terminals receive:
```
< {"user_name":"charlie","message":"This is awesome! Love the real-time updates!"}
```

### Group Chat Implementation Details

- **WebSocket Handler**: `src/websocket/group.rs` — Manages group chat connections and message routing
- **Group REST API**: `src/group/handler.rs` — CRUD operations for groups (create, list, get by ID)
- **State Management**: `GroupState` with broadcast channel for pub/sub messaging
- **Broadcast Channel**: Single `broadcast::Sender` delivers messages to all subscribed receivers
- **Database**: Groups stored in PostgreSQL with name, description, and auto-generated UUID
- **Connection Cleanup**: Graceful cleanup when user disconnects

### Key Differences: Private Chat vs Group Chat

| Feature | Private Chat | Group Chat |
|---------|-------------|-----------|
| **Participants** | 2 users (1-to-1) | Multiple users (N-to-N) |
| **Message Routing** | HashMap lookup per receiver | Broadcast to all subscribers |
| **Broadcast Channel** | Per-user channel | Single group channel |
| **Connection Tracking** | HashMap of user→channel mappings | Single broadcast sender |
| **Use Case** | Direct messaging | Team discussions, communities |

For implementation details and documentation, see:
- `src/websocket/group.rs` — WebSocket group chat handler
- `src/group/handler.rs` — Group management API

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

