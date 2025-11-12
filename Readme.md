This repository is a small Rust project scaffold intended as the starting point for implementing an HTTP API using the Axum framework.

## Goals

- Provide a minimal, well-documented project that demonstrates how to build an HTTP API with Axum and Tokio.
- Show recommended project layout, common dependencies, and quick start instructions.
- Describe next steps for implementing endpoints, middleware, configuration, tests, and deployment.

## Prerequisites

- Rust toolchain (stable) installed. See https://rustup.rs
- Cargo available on your PATH (installed with rustup)
- (Optional) Docker for containerized builds and deployment

## Quick start

1. Clone the repository and change into the project directory.

   ```bash
   cd {your_path}/example-axum-api
   ```

- install crate `sqlx-cli`
  ```bash
  cargo install sqlx-cli
  ```

- create dev.toml file, then inside toml file
  ```bash
  name = "development"
  
  [database]
  url="postgres://{user_name}:{password}@{host}:{port}/{database_name}"
  host = {host}
  port = {port}
  user = {user_name}
  password = {password}
  name = {database_name}
  max_connection = 10
  min_connection = 5
  acquire_timeout = 5
  idle_timeout = 60
  ```

- then export url 
  unix/linux
  ```bash
  export DATABASE_URL=$(awk -F'"' '/url=/ {print $2}' dev.toml)
  ```
  windows
  ```bash
  set DATABASE_URL=$(awk -F'"' '/url=/ {print $2}' dev.toml)
  ```

- create new database
  ```bash
  sqlx database create
  ```
