use std::sync::Arc;

use axum::{
    Form,
    extract::{Path, State},
    response::{IntoResponse, Json},
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::{Error, Pool, Postgres, Row, postgres::PgRow};

use crate::{
    app_state::AppState,
    auth::util::{MetaResponse, StatusCodeExt},
};

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct Group {
    pub group_id: String,
    pub name: String,
    pub description: Option<String>,
}

impl IntoResponse for Group {
    fn into_response(self) -> axum::response::Response {
        let status = StatusCode::OK;
        (status, Json(self)).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupResponse {
    pub meta: MetaResponse,
    pub data: Group,
}

impl IntoResponse for GroupResponse {
    fn into_response(self) -> axum::response::Response {
        let status = StatusCode::OK;
        (status, Json(self)).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupsResponse {
    pub meta: MetaResponse,
    pub data: Vec<Group>,
}

impl IntoResponse for GroupsResponse {
    fn into_response(self) -> axum::response::Response {
        let status = StatusCode::OK;
        (status, Json(self)).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupParam {
    pub name: String,
    pub description: Option<String>,
}

pub async fn create(pool: &Pool<Postgres>, name: &str, desc: &str) -> Result<Group, Error> {
    let mut tx = pool.begin().await?;
    let group_id = uuid::Uuid::new_v4().to_string();
    let description = if !desc.is_empty() {
        desc.to_string()
    } else {
        "".to_string()
    };

    let sql = "insert into groups (group_id, name, description) values ($1, $2, $3)";
    sqlx::query(sql)
        .bind(group_id.clone())
        .bind(name)
        .bind(description.clone())
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(Group {
        group_id: group_id,
        name: name.to_string(),
        description: Some(description),
    })
}

pub async fn get_by_id(pool: &Pool<Postgres>, group_id: &str) -> Result<Group, Error> {
    let sql = "select group_id, name, description from groups where group_id = $1";
    let result = sqlx::query(sql)
        .bind(group_id)
        .map(|data: PgRow| Group {
            group_id: data.get("group_id"),
            name: data.get("name"),
            description: data.get("description"),
        })
        .fetch_optional(pool)
        .await?;

    match result {
        Some(data) => Ok(Group {
            group_id: data.group_id,
            name: data.name.to_string(),
            description: Some(data.description.unwrap_or_default()),
        }),
        None => Err(Error::RowNotFound),
    }
}

pub async fn get_all(pool: &Pool<Postgres>, page: i32) -> Result<Vec<Group>, Error> {
    let sql =
        "select group_id, name, description from groups order by name desc limit 10 offset $1";
    let offset = if page > 0 { (page - 1) * 10 } else { 0 };

    let groups = sqlx::query(sql)
        .bind(offset)
        .map(|data: PgRow| Group {
            group_id: data.get("group_id"),
            name: data.get("name"),
            description: data.get("description"),
        })
        .fetch_all(pool)
        .await?;
    Ok(groups)
}

pub async fn create_group_handler(
    State(state): State<Arc<AppState>>,
    Form(req): Form<GroupParam>,
) -> Result<GroupResponse, MetaResponse> {
    let result = create(
        &state.pool,
        &req.name,
        req.description.as_deref().unwrap_or(""),
    )
    .await
    .map_err(|e| MetaResponse {
        code: StatusCode::BAD_REQUEST.to_i32(),
        message: e.to_string(),
    })?;
    Ok(GroupResponse {
        meta: MetaResponse {
            code: StatusCode::OK.to_i32(),
            message: "Success".to_string(),
        },
        data: result,
    })
}

pub async fn groups_handler(
    State(state): State<Arc<AppState>>,
    Path(page): Path<i32>,
) -> Result<GroupsResponse, MetaResponse> {
    let result = get_all(&state.pool, page).await.map_err(|e| MetaResponse {
        code: StatusCode::BAD_REQUEST.to_i32(),
        message: e.to_string(),
    })?;
    println!("{:?}", result);
    Ok(GroupsResponse {
        meta: MetaResponse {
            code: StatusCode::OK.to_i32(),
            message: "Success".to_string(),
        },
        data: result,
    })
}

#[cfg(test)]
mod tests_group {
    use std::sync::Arc;

    use axum::{
        Router,
        routing::{get, post},
    };
    use axum_test::TestServer;
    use http::StatusCode;

    use crate::{
        app_state::AppState,
        auth::util::random_name,
        config::connection::ConnectionBuilder,
        group::handler::{GroupParam, create_group_handler, groups_handler},
    };

    #[tokio::test]
    async fn test_create_new() {
        let builder = ConnectionBuilder(String::from("dev.toml"));
        let pool = ConnectionBuilder::new(&builder)
            .await
            .expect("Failed to connect database");

        let state = Arc::new(AppState::new(pool));

        let app = Router::new()
            .route("/api/groups", post(create_group_handler))
            .with_state(state);
        let name = random_name();
        let body = GroupParam {
            name: name,
            description: Some("".to_string()),
        };
        let server = TestServer::new(app).expect("Failed start server");
        let response = server.post("/api/groups").form(&body).await;
        assert_eq!(response.status_code(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_all() {
        let builder = ConnectionBuilder(String::from("dev.toml"));
        let pool = ConnectionBuilder::new(&builder)
            .await
            .expect("Failed to connect database");

        let state = Arc::new(AppState::new(pool));

        let app = Router::new()
            .route("/api/groups/{page}", get(groups_handler))
            .with_state(state);

        let server = TestServer::new(app).expect("Failed start server");
        let response = server.get("/api/groups/1").await;
        assert_eq!(response.status_code(), StatusCode::OK);
    }
}
