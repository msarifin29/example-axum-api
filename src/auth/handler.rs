use crate::{
    AppState,
    auth::{
        extractors::AuthUser,
        jwt::{create_access_token, create_refresh_token},
        user::{
            NewUser, User, UserResponse, add, delete_user, get_by_user_name, get_users,
            update_password,
        },
        util::{MetaResponse, StatusCodeExt, passwords_match},
    },
};
use axum::{
    Form,
    extract::{Query, State},
    response::{IntoResponse, Json, Response},
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(serde::Serialize)]
pub struct AuthResponse {
    pub meta: MetaResponse,
    pub data: User,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
}
impl IntoResponse for AuthResponse {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginParam {
    pub user_name: String,
    pub password: String,
}

pub async fn register_handler(
    State(state): State<Arc<AppState>>,
    Form(req): Form<NewUser>,
) -> Result<AuthResponse, MetaResponse> {
    let sql = "select user_name from users where user_name = $1";
    let existing = sqlx::query(sql)
        .bind(req.user_name.clone())
        .fetch_optional(state.pool.as_ref())
        .await;

    if let Ok(Some(_)) = existing {
        MetaResponse {
            code: StatusCode::BAD_REQUEST.to_i32(),
            message: "User name already registered".to_string(),
        };
    }

    let result = add(&state.pool, req).await.map_err(|e| MetaResponse {
        code: StatusCode::BAD_REQUEST.to_i32(),
        message: format!("Failed to register: {}", e.to_string()),
    })?;

    let access_token = create_access_token(&state.jwt_config, &result.user_id, &result.email).ok();
    let refresh_token =
        create_refresh_token(&state.jwt_config, &result.user_id, &result.email).ok();

    Ok(AuthResponse {
        meta: MetaResponse {
            code: StatusCode::OK.to_i32(),
            message: String::from("Success"),
        },
        data: result,
        access_token: access_token,
        refresh_token: refresh_token,
    })
}

pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    Form(req): Form<LoginParam>,
) -> Result<AuthResponse, MetaResponse> {
    let result = get_by_user_name(req.user_name, &state.pool)
        .await
        .map_err(|_| MetaResponse {
            code: StatusCode::NOT_FOUND.to_i32(),
            message: "Invalid user name or password".to_string(),
        })?;

    let is_err = passwords_match(&req.password, &result.password);
    if let Err(_) = is_err {
        MetaResponse {
            code: StatusCode::NOT_FOUND.to_i32(),
            message: "Invalid user name or password".to_string(),
        };
    }

    let access_token = create_access_token(&state.jwt_config, &result.user_id, &result.email).ok();
    let refresh_token =
        create_refresh_token(&state.jwt_config, &result.user_id, &result.email).ok();

    let data = User {
        user_id: result.user_id,
        user_name: result.user_name,
        email: result.email,
    };
    Ok(AuthResponse {
        meta: MetaResponse {
            code: StatusCode::OK.to_i32(),
            message: String::from("Success"),
        },
        data: data,
        access_token: access_token,
        refresh_token: refresh_token,
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
    pub password: String,
}

pub async fn update_password_handler(
    AuthUser(user): AuthUser,
    State(state): State<Arc<AppState>>,
    Form(req): Form<UpdatePasswordParam>,
) -> MetaResponse {
    let result = update_password(&user.user_id, &req.password, &state.pool).await;
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
    AuthUser(user): AuthUser,
    State(state): State<Arc<AppState>>,
) -> MetaResponse {
    let result = delete_user(&user.user_id, &state.pool).await;
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

    use axum::{Router, body::Body, http::StatusCode};
    use http::Request;
    use tower::ServiceExt;

    use crate::{
        AppState,
        auth::{
            handler::{LoginParam, NewUser, UpdatePasswordParam},
            util::random_name,
        },
        routes::routes,
    };
    use std::{sync::Arc, usize};

    pub async fn get_access_token(app: &Router, user_name: &str, password: &str) -> Option<String> {
        let body = LoginParam {
            user_name: user_name.to_string(),
            password: password.to_string(),
        };

        let server = TestServer::new(app.clone()).unwrap();

        let response = server.post("/api/auth/login").form(&body).await;
        response.assert_status_ok();

        let form_data = format!("user_name={}&password={}", user_name, password);

        let request = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(form_data))
            .expect("Failed request response");

        let response = app
            .clone()
            .oneshot(request)
            .await
            .expect("Failed call request");
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed convert to bytes");

        let login_response: serde_json::Value =
            serde_json::from_slice(&body).expect("Failed to convert serde json");

        let token = login_response["access_token"]
            .as_str()
            .expect("failed to get access token");

        Some(token.to_string())
    }

    #[tokio::test]
    async fn test_register_user() {
        let state = Arc::new(AppState::test().await);

        let app = routes(state);

        let server = TestServer::new(app).unwrap();

        let user_name = random_name().to_string();
        let email = format!("{}.example.@mail.com", user_name.clone());
        let body = NewUser {
            user_name: user_name,
            email: email,
            password: "123456".to_string(),
        };
        let response = server.post("/api/auth/register").form(&body).await;
        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_register_duplicate_username() {
        let state = Arc::new(AppState::test().await);

        let app = routes(state);

        let server = TestServer::new(app).unwrap();
        let user_name = "Jordan".to_string();
        let email = format!("{}.example.@mail.com", user_name.clone());
        let body = NewUser {
            user_name: user_name,
            email: email,
            password: "123456".to_string(),
        };

        let response = server.post("/api/auth/register").form(&body).await;
        response.assert_status_bad_request();
    }

    #[tokio::test]
    async fn test_login_user() {
        let state = Arc::new(AppState::test().await);

        let app = routes(state);

        let server = TestServer::new(app).unwrap();

        let body = LoginParam {
            user_name: "Jordan".to_string(),
            password: "123456".to_string(),
        };
        let response = server.post("/api/auth/login").form(&body).await;
        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_login_invalid_user_name() {
        let state = Arc::new(AppState::test().await);

        let app = routes(state);

        let server = TestServer::new(app).unwrap();

        let body = LoginParam {
            user_name: "".to_string(),
            password: "123456".to_string(),
        };
        let response = server.post("/api/auth/login").form(&body).await;
        assert_eq!(response.status_code(), StatusCode::NOT_FOUND)
    }

    #[tokio::test]
    async fn test_get_users() {
        let state = Arc::new(AppState::test().await);

        let app = routes(state);

        let server = TestServer::new(app.clone()).unwrap();

        let user_name = "Jordan".to_string();
        let password = "123456".to_string();
        let token = get_access_token(&app, &user_name, &password).await.unwrap();

        let response = server
            .get("/api/users?page=1")
            .add_header("Authorization", format!("Bearer {}", token))
            .await;
        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_get_users_by_name() {
        let state = Arc::new(AppState::test().await);

        let app = routes(state);

        let server = TestServer::new(app.clone()).unwrap();

        let user_name = "Jordan".to_string();
        let password = "123456".to_string();
        let token = get_access_token(&app, &user_name, &password).await.unwrap();

        let response = server
            .get("/api/users?page=1&user_name=x")
            .add_header("Authorization", format!("Bearer {}", token))
            .await;
        response.assert_status_ok();
    }

    #[tokio::test]
    async fn test_get_users_unauthorized() {
        let state = Arc::new(AppState::test().await);

        let app = routes(state);

        let server = TestServer::new(app.clone()).unwrap();

        let response = server
            .get("/api/users?page=1&user_name=x")
            .add_header("Authorization", format!("Bearer {}", "token"))
            .await;
        response.assert_status_unauthorized();
    }

    #[tokio::test]
    async fn test_update_password() {
        let state = Arc::new(AppState::test().await);

        let app = routes(state);

        let server = TestServer::new(app.clone()).unwrap();

        let user_name = random_name().to_string();
        let email = format!("{}.example.@mail.com", user_name.clone());
        let password = "123456".to_string();
        let body = NewUser {
            user_name: user_name.clone(),
            email: email,
            password: password.clone(),
        };

        let response = server.post("/api/auth/register").form(&body).await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let new_password = random_name().to_string();
        let token = get_access_token(&app.clone(), &user_name, &password)
            .await
            .unwrap();

        let param = UpdatePasswordParam {
            password: new_password,
        };
        let response = server
            .put("/api/auth/update-password")
            .add_header("Authorization", format!("Bearer {}", token))
            .form(&param)
            .await;
        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_update_password_failed() {
        let state = Arc::new(AppState::test().await);

        let app = routes(state);

        let server = TestServer::new(app.clone()).unwrap();

        let user_name = random_name().to_string();
        let email = format!("{}.example.@mail.com", user_name.clone());
        let password = "123456".to_string();
        let body = NewUser {
            user_name: user_name.clone(),
            email: email,
            password: password.clone(),
        };

        let response = server.post("/api/auth/register").form(&body).await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let token = get_access_token(&app.clone(), &user_name, &password)
            .await
            .unwrap();

        let param = UpdatePasswordParam { password: password };
        let response = server
            .put("/api/auth/update-password")
            .add_header("Authorization", format!("Bearer {}", token))
            .form(&param)
            .await;
        assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_delete_user() {
        let state = Arc::new(AppState::test().await);

        let app = routes(state);
        let server = TestServer::new(app.clone()).unwrap();

        let user_name = random_name().to_string();
        let email = format!("{}.example.@mail.com", user_name.clone());
        let password = "123456".to_string();
        let body = NewUser {
            user_name: user_name.clone(),
            email: email,
            password: password.clone(),
        };

        let response = server.post("/api/auth/register").form(&body).await;
        assert_eq!(response.status_code(), StatusCode::OK);

        let token = get_access_token(&app.clone(), &user_name, &password)
            .await
            .unwrap();

        let response = server
            .delete("/api/auth/delete-account")
            .add_header("Authorization", format!("Bearer {}", token))
            .await;
        assert_eq!(response.status_code(), StatusCode::OK);
    }
}
