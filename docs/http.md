# HTTP API usage (curl examples)

This document shows how to call the project's HTTP endpoints using `curl`. It covers authentication (register/login), user endpoints (list, update password, delete), and group endpoints (create, list).

Base URL (local dev):

```text
http://127.0.0.1:3000
```

Prerequisites
- Server running: `cargo run`

---

## Authentication

### Register (create user)

POST /api/auth/register

Form fields: `user_name`, `email`, `password`

Example:

```bash
curl -s -X POST http://127.0.0.1:3000/api/auth/register \
-H "Content-Type: application/x-www-form-urlencoded" \
-d "user_name=jdoe&email=jdoe@example.com&password=secret123"
```

Successful response contains `data.user_id`, `access_token`, and `refresh_token`.

### Login

POST /api/auth/login

Form fields: `user_name`, `password`

Example:

```bash
curl -s -X POST http://127.0.0.1:3000/api/auth/login \
-H "Content-Type: application/x-www-form-urlencoded" \
-d "user_name=jdoe&password=secret123"
```

The response contains `access_token` (use this bearer token to call protected endpoints).

---

## Protected user endpoints

All protected endpoints require the header `Authorization: Bearer {ACCESS_TOKEN}`.

### List users

GET /api/users?page={page}&user_name={optional}

Example (page 1):

```bash
curl -s "http://127.0.0.1:3000/api/users?page=1" \
-H "Authorization: Bearer {ACCESS_TOKEN}"
```

Optional filter by `user_name`:

```bash
curl -s "http://127.0.0.1:3000/api/users?page=1&user_name=J" \
-H "Authorization: Bearer {ACCESS_TOKEN}"
```

### Update password

PUT /api/auth/update-password

Form fields: `password` (new password)

Example:

```bash
curl -s -X PUT http://127.0.0.1:3000/api/auth/update-password \
-H "Content-Type: application/x-www-form-urlencoded" \
-H "Authorization: Bearer {ACCESS_TOKEN}" \
-d "password=newsecret"
```

### Delete account

DELETE /api/auth/delete-account

Example:

```bash
curl -s -X DELETE http://127.0.0.1:3000/api/auth/delete-account \
-H "Authorization: Bearer {ACCESS_TOKEN}"
```

---

## Groups

### Create a group

POST /api/groups

Form fields: `name`, `description` (optional)

Example:

```bash
curl -s -X POST http://127.0.0.1:3000/api/groups \
-H "Content-Type: application/x-www-form-urlencoded" \
-H "Authorization: Bearer {ACCESS_TOKEN}" \
-d "name=DevChat&description=Developers chatting"
```

Response contains created `group_id` under `data.group_id`.

### List groups (paginated)

GET /api/groups/{page}

Example (page 1):

```bash
curl -s http://127.0.0.1:3000/api/groups/1 \
-H "Authorization: Bearer {ACCESS_TOKEN}"
```

Note: If your project routes use `/api/groups` without a page path, try `http://127.0.0.1:3000/api/groups?page=1` instead. The project contains `groups_handler` which expects a page parameter.

---

## Notes & Troubleshooting

- If a protected request returns `401 Unauthorized`, ensure your token is correct and not expired. Tokens in this test project are generated as access tokens from `create_access_token`.
- Check `dev.toml` for the bound IP/port (default `127.0.0.1:3000`).
- If endpoints return unexpected errors, inspect server logs for details (missing DB, migration not applied, etc.).

