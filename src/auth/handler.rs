use crate::auth::{
    user::{NewUser, User, UserResponse, add, get_users},
    util::{MetaResponse, StatusCodeExt},
};
use axum::{
    Form,
    extract::{Query, State},
    response::{IntoResponse, Json, Response},
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use std::sync::Arc;

#[derive(serde::Serialize)]
pub struct SingleUserResponse {
    pub meta: MetaResponse,
    pub data: User,
}
impl IntoResponse for SingleUserResponse {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.meta.code as u16).unwrap_or(StatusCode::OK);

        (status, Json(self)).into_response()
    }
}

#[derive(Debug, Deserialize)]
pub struct GetUsersQuery {
    #[serde(default)]
    pub page: i32,
    #[serde(default)]
    pub user_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UsersResponse {
    pub meta: MetaResponse,
    pub data: UserResponse,
}
impl IntoResponse for UsersResponse {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.meta.code as u16).unwrap_or(StatusCode::OK);
        (status, Json(self)).into_response()
    }
}

pub async fn add_user_handler(
    State(pool): State<Arc<Pool<Postgres>>>,
    Form(req): Form<NewUser>,
) -> Result<SingleUserResponse, MetaResponse> {
    let result = add(&pool, req).await.map_err(|e| MetaResponse {
        code: StatusCode::BAD_REQUEST.to_i32(),
        message: e.to_string(),
    })?;

    Ok(SingleUserResponse {
        meta: MetaResponse {
            code: StatusCode::OK.to_i32(),
            message: String::from("Success"),
        },
        data: result,
    })
}

pub async fn get_users_handler(
    State(pool): State<Arc<Pool<Postgres>>>,
    Query(params): Query<GetUsersQuery>,
) -> Result<UsersResponse, MetaResponse> {
    let page = params.page;
    let user_name = params.user_name.unwrap_or_default();
    let result = get_users(page, &user_name, &pool)
        .await
        .map_err(|e| MetaResponse {
            code: StatusCode::BAD_REQUEST.to_i32(),
            message: e.to_string(),
        })?;

    Ok(UsersResponse {
        meta: MetaResponse {
            code: StatusCode::OK.to_i32(),
            message: String::from("Success"),
        },
        data: result,
    })
}

#[cfg(test)]
mod tests_user {
    use axum_test::TestServer;

    use axum::{
        Router,
        routing::{get, post},
    };

    use crate::auth::handler::{NewUser, add_user_handler, get_users_handler};
    use crate::config::connection::ConnectionBuilder;
    use sqlx::{Pool, Postgres};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_add_user() {
        let builder = ConnectionBuilder(String::from("dev.toml"));
        let pool = ConnectionBuilder::new(&builder)
            .await
            .expect("Failed to connect to database");

        let db_state: Arc<Pool<Postgres>> = Arc::new(pool);

        let app = Router::new()
            .route("/api/user", post(add_user_handler))
            .with_state(db_state);

        let server = TestServer::new(app).unwrap();
        let body = NewUser {
            user_name: "jhonkei".to_string(),
            email: "jhonkei.example.@mail.com".to_string(),
            password: "123456".to_string(),
        };
        let response = server.post("/api/user").form(&body).await;
        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_add_user_duplicate_username() {
        let builder = ConnectionBuilder(String::from("dev.toml"));
        let pool = ConnectionBuilder::new(&builder)
            .await
            .expect("Failed to connect to database");
        let db_state: Arc<Pool<Postgres>> = Arc::new(pool);

        let app = Router::new()
            .route("/api/users", post(add_user_handler))
            .with_state(db_state);

        let server = TestServer::new(app).unwrap();
        let body = NewUser {
            user_name: "jhonkei".to_string(),
            email: "jhonkei.example.@mail.com".to_string(),
            password: "123456".to_string(),
        };
        let response = server.post("/api/users").form(&body).await;
        response.assert_status_bad_request();
    }

    #[tokio::test]
    async fn test_get_users() {
        let builder = ConnectionBuilder(String::from("dev.toml"));
        let pool = ConnectionBuilder::new(&builder)
            .await
            .expect("Failed to connect to database");
        let db_state: Arc<Pool<Postgres>> = Arc::new(pool);

        let app = Router::new()
            .route("/api/users", get(get_users_handler))
            .with_state(db_state);

        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/users?page=1&user_name=J").await;
        response.assert_status_ok();
    }
}
