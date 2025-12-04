# Testing WebSocket chat (private & group) with websocat

This document explains how to test the WebSocket-based chat features (private 1:1 chat and group chat)
implemented under `src/websocket` using `websocat`.

These instructions assume the server is running locally at `http://127.0.0.1:3000`.

Prerequisites
 - Server running: `cargo run`
 - `websocat` installed 

Install websocat (example):

```bash
cargo install websocat
```

## 1. Private (end-to-end) chat

Private chat endpoint: `ws://127.0.0.1:3000/chat`

Behavior summary (server-side):
- The handler extracts the authenticated sender from the `Authorization` header (expects `Bearer {sender_id}`).
- The handler also expects a `receiver_id` HTTP header specifying the target user id.
- Both participants should connect (each with their own Authorization header). Messages sent by one user are routed to the other.

### Step A — Create two users

Create Alice and Bob and capture their user IDs:

```bash
# Register user 1
curl -s -X POST http://127.0.0.1:3000/api/auth/register \
-H "Content-Type: application/x-www-form-urlencoded" \
-d "user_name=alice&email=alice@example.com&password=pass123"

# Login user 1
curl -s -X POST http://127.0.0.1:3000/api/auth/login \
-H "Content-Type: application/x-www-form-urlencoded" \
-d "user_name=alice&password=pass123"

# Register user 2
curl -s -X POST http://127.0.0.1:3000/api/auth/register \
-H "Content-Type: application/x-www-form-urlencoded" \
-d "user_name=bobmarley&email=bobmarley@example.com&password=pass123"

# Login user 2
curl -s -X POST http://127.0.0.1:3000/api/auth/login \
-H "Content-Type: application/x-www-form-urlencoded" \
-d "user_name=bobmarley&password=pass123"

```

### Step B — Open two websocat sessions (Alice ↔ Bob)

Open Terminal A (Alice):

```bash
websocat "ws://127.0.0.1:3000/chat" \
-H "Authorization: Bearer {ACCESS_TOKEN}" \
-H "receiver_id: {USER2}"
```

Open Terminal B (Bob):

```bash
websocat ws://127.0.0.1:3000/chat \
-H "Authorization: Bearer {ACCESS_TOKEN}" \
-H "receiver_id: {USER1}"
```

Now type messages in Terminal A and they should appear in Terminal B as JSON messages like:

```json
{"sender_user":{"user_id":"<USER_ID>","user_name":"Alice","email":"alice@example.com"},"receiver_user":{"user_id":"<USER_ID>","user_name":"bobmarley","email":"bobmarley@example.com"},"message":"Hello Bob!\n","timestamp":1700XXXXX}
```

And replies sent from Terminal B will appear in Terminal A.

Troubleshooting
- If you get `400` or `Invalid user_id` errors, ensure both IDs exist in the DB and you used the correct endpoints to create them.
- Verify the server logs for validation errors.
- Make sure each websocat command includes the correct `Authorization` header (Bearer token must be exactly the user_id in this example).

## 2. Group chat (broadcast to group members)

Group chat endpoint: `ws://127.0.0.1:3000/group-chat`

Behavior summary (server-side):
- Validates `user_id` and `group_id` with the DB
- Uses a single broadcast channel per group; all connected members receive broadcast messages
- When a member joins, a welcome message is broadcast

### Step A — Create  a group

```bash
curl -s -X POST http://127.0.0.1:3000/api/groups \
-H "Content-Type: application/x-www-form-urlencoded" \
-H "Authorization: Bearer {ACCESS_TOKEN}" \
-d "name=DevChat&description=Developers chatting"
```

### Step B — Connect multiple members to the same group


```bash
websocat "ws://127.0.0.1:3000/group-chat" \
-H "Authorization: Bearer {ACCESS_TOKEN}" \
-H "group_id:{GROUP_ID}"
```

Type a message in any terminal and all connected members should receive a JSON payload:

```json
{"id": "12345", "name":"alice","message":"Hello everyone!"}
```


## 4) Troubleshooting checklist

- Ensure the server is running and bound to the address/port in `dev.toml` (default `127.0.0.1:3000`).
- Verify DB migrations applied and users/groups exist.
- For private chat, ensure both participants are connected; messages are routed only when the receiver has an active connection.
- Check server logs for panics or errors — they often reveal missing headers or validation failures.
