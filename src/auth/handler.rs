use crate::{
    AppState,
    auth::{
        user::{NewUser, User, UserResponse, add, delete_user, get_users, update_password},
        util::{MetaResponse, StatusCodeExt},
    },
};
use axum::{
    Form,
    extract::{Path, Query, State},
    response::{IntoResponse, Json, Response},
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
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
    State(state): State<Arc<AppState>>,
    Form(req): Form<NewUser>,
) -> Result<SingleUserResponse, MetaResponse> {
    let result = add(&state.pool, req).await.map_err(|e| MetaResponse {
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
    State(state): State<Arc<AppState>>,
    Query(params): Query<GetUsersQuery>,
) -> Result<UsersResponse, MetaResponse> {
    let page = params.page;
    let user_name = params.user_name.unwrap_or_default();
    let result = get_users(page, &user_name, &state.pool)
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

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdatePasswordParam {
    pub user_id: String,
    pub password: String,
}

pub async fn update_password_handler(
    State(state): State<Arc<AppState>>,
    Form(req): Form<UpdatePasswordParam>,
) -> MetaResponse {
    let result = update_password(&req.user_id, &req.password, &state.pool).await;
    match result {
        Ok(_) => MetaResponse {
            code: StatusCode::OK.to_i32(),
            message: String::from("Success"),
        },
        Err(e) => MetaResponse {
            code: StatusCode::BAD_REQUEST.to_i32(),
            message: e.to_string(),
        },
    }
}

pub async fn delete_user_handler(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> MetaResponse {
    let result = delete_user(&user_id, &state.pool).await;
    match result {
        Ok(_) => MetaResponse {
            code: StatusCode::OK.to_i32(),
            message: String::from("Success"),
        },
        Err(e) => MetaResponse {
            code: StatusCode::BAD_REQUEST.to_i32(),
            message: e.to_string(),
        },
    }
}

#[cfg(test)]
mod tests_user {
    use axum_test::TestServer;

    use axum::{
        Router,
        body::Body,
        routing::{delete, get, post, put},
    };
    use http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::config::connection::ConnectionBuilder;
    use crate::{
        AppState,
        auth::{
            handler::{
                NewUser, UpdatePasswordParam, add_user_handler, delete_user_handler,
                get_users_handler, update_password_handler,
            },
            util::random_name,
        },
    };
    use std::{sync::Arc, usize};

    #[tokio::test]
    async fn test_add_user() {
        let builder = ConnectionBuilder(String::from("dev.toml"));
        let pool = ConnectionBuilder::new(&builder)
            .await
            .expect("Failed to connect to database");

        let db_state = Arc::new(AppState {
            pool: Arc::new(pool),
        });

        let app = Router::new()
            .route("/api/users", post(add_user_handler))
            .with_state(db_state);

        let server = TestServer::new(app).unwrap();
        let user_name = random_name().to_string();
        let email = format!("{}.example.@mail.com", user_name.clone());
        let body = NewUser {
            user_name: user_name,
            email: email,
            password: "123456".to_string(),
        };
        let response = server.post("/api/users").form(&body).await;
        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_add_user_duplicate_username() {
        let builder = ConnectionBuilder(String::from("dev.toml"));
        let pool = ConnectionBuilder::new(&builder)
            .await
            .expect("Failed to connect to database");
        let db_state = Arc::new(AppState {
            pool: Arc::new(pool),
        });

        let app = Router::new()
            .route("/api/users", post(add_user_handler))
            .with_state(db_state);

        let server = TestServer::new(app).unwrap();
        let user_name = random_name().to_string();
        let email = format!("{}.example.@mail.com", user_name.clone());
        let body = NewUser {
            user_name: user_name,
            email: email,
            password: "123456".to_string(),
        };
        let response = server.post("/api/users").form(&body).await;
        response.assert_status_ok();

        let response = server.post("/api/users").form(&body).await;
        response.assert_status_bad_request();
    }

    #[tokio::test]
    async fn test_get_users() {
        let builder = ConnectionBuilder(String::from("dev.toml"));
        let pool = ConnectionBuilder::new(&builder)
            .await
            .expect("Failed to connect to database");
        let db_state = Arc::new(AppState {
            pool: Arc::new(pool),
        });

        let app = Router::new()
            .route("/api/users", get(get_users_handler))
            .with_state(db_state);

        let server = TestServer::new(app).unwrap();

        let response = server.get("/api/users?page=1&user_name=J").await;
        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_update_password() {
        let builder = ConnectionBuilder(String::from("dev.toml"));
        let pool = ConnectionBuilder::new(&builder)
            .await
            .expect("Failed to connect to database");
        let db_state = Arc::new(AppState {
            pool: Arc::new(pool),
        });

        let app = Router::new()
            .route("/api/users", post(add_user_handler))
            .route("/api/users", put(update_password_handler))
            .with_state(db_state);

        let server = TestServer::new(app.clone()).unwrap();
        let name = random_name().to_string();
        let form_data = format!(
            "user_name={}&email={}@example.com&password=pass123",
            name, name
        );
        let request = Request::builder()
            .method("POST")
            .uri("/api/users")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(form_data))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();

        let create_response: serde_json::Value = serde_json::from_slice(&body).unwrap();

        let user_id = create_response["data"]["user_id"]
            .as_str()
            .expect("user_id not found");

        let param = UpdatePasswordParam {
            user_id: user_id.to_string(),
            password: "65431".to_string(),
        };
        let response = server.put("/api/users").form(&param).await;
        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_delete_user() {
        let builder = ConnectionBuilder(String::from("dev.toml"));
        let pool = ConnectionBuilder::new(&builder)
            .await
            .expect("Failed to connect to database");
        let db_state = Arc::new(AppState {
            pool: Arc::new(pool),
        });

        let app = Router::new()
            .route("/api/users", post(add_user_handler))
            .route("/api/users/{user_id}", delete(delete_user_handler))
            .with_state(db_state);

        let server = TestServer::new(app.clone()).unwrap();

        let form_data = "user_name=testdelete&email=crud@example.com&password=pass123";
        let request = Request::builder()
            .method("POST")
            .uri("/api/users")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(form_data))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();

        let create_response: serde_json::Value = serde_json::from_slice(&body).unwrap();

        let user_id = create_response["data"]["user_id"]
            .as_str()
            .expect("user_id not found");
        let path = format!("/api/users/{}", user_id);
        let response = server.delete(&path).await;
        assert_eq!(response.status_code(), StatusCode::OK);
    }
}
