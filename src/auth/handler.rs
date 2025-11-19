use crate::auth::{
    user::{NewUser, User, add},
    util::{MetaResponse, StatusCodeExt},
};
use axum::{
    Form,
    extract::State,
    response::{IntoResponse, Json, Response},
};
use http::StatusCode;
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

#[cfg(test)]
mod tests_user {
    use axum_test::TestServer;

    use crate::config::connection::pg_test;
    use axum::{Router, routing::post};

    use crate::auth::handler::{NewUser, add_user_handler};
    use sqlx::{Pool, Postgres};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_add_user() {
        let pool = pg_test().await.expect("Failed to connect to database");
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
        let pool = pg_test().await.expect("Failed to connect to database");
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
        response.assert_status_bad_request();
    }
}
