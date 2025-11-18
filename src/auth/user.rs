use std::{borrow::Cow, error::Error as fmt_error};

use serde::{Deserialize, Serialize};
use sqlx::{Error, Pool, Postgres, Row, postgres::PgRow};
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::auth::util::{MsgError, hash_password, passwords_match};

#[derive(Debug, Serialize, Deserialize, Validate)]
#[validate(context = UserContext,
schema(
    function="unique_name",
    skip_on_field_errors=false,
    code="username",
    use_context,
))]
pub struct NewUser {
    #[validate(length(min = 6, max = 30, code = "username"))]
    pub user_name: String,
    #[validate(email)]
    pub email: String,
    pub password: String,
}

impl NewUser {
    pub fn new(user_name: String, email: String, password: String) -> NewUser {
        NewUser {
            user_name: user_name,
            email: email,
            password: password,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub page: i32,
    pub data: Vec<User>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub user_name: String,
    pub email: String,
}

pub struct UserContext {
    pub user_name: String,
}

fn unique_name(user: &NewUser, context: &UserContext) -> Result<(), ValidationError> {
    if user.user_name == context.user_name {
        return Err(
            ValidationError::new("username").with_message(Cow::from(format!(
                "cannot register using user name {}, user name already exists",
                user.user_name,
            ))),
        );
    }

    Ok(())
}

pub async fn add(pg: &Pool<Postgres>, new_user: NewUser) -> Result<(), Error> {
    let mut tx = pg.begin().await?;

    let script = "insert into users(user_id, user_name, email, password) values($1, $2, $3, $4)";
    let uid = Uuid::new_v4();

    sqlx::query(script)
        .bind(uid.to_string())
        .bind(new_user.user_name)
        .bind(new_user.email)
        .bind(new_user.password)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn get_by_user_name(name: String, pool: &Pool<Postgres>) -> Result<NewUser, Error> {
    let result = sqlx::query("select user_name, email, password from users where user_name = $1")
        .bind(name.to_string())
        .map(|data: PgRow| NewUser {
            user_name: data.get("user_name"),
            email: data.get("email"),
            password: data.get("password"),
        })
        .fetch_optional(pool)
        .await?;

    match result {
        Some(user) => Ok(NewUser::new(user.user_name, user.email, user.password)),
        None => Err(Error::RowNotFound),
    }
}

pub async fn get_by_user_id(user_id: String, pool: &Pool<Postgres>) -> Result<NewUser, Error> {
    let result = sqlx::query("select user_name, email, password from users where user_id = $1")
        .bind(user_id.to_string())
        .map(|data: PgRow| NewUser {
            user_name: data.get("user_name"),
            email: data.get("email"),
            password: data.get("password"),
        })
        .fetch_optional(pool)
        .await?;

    match result {
        Some(user) => Ok(NewUser::new(user.user_name, user.email, user.password)),
        None => Err(Error::RowNotFound),
    }
}

async fn new_password(
    user_id: &str,
    new_pwd: &str,
    pool: &Pool<Postgres>,
) -> Result<(String, bool), MsgError> {
    let user = get_by_user_id(user_id.to_string(), pool)
        .await
        .map_err(|e| MsgError(format!("Failed to get user: {}", e)))?;

    let match_password = passwords_match(&user.password, new_pwd)
        .map_err(|e| MsgError(format!("Failed to compare passwords: {}", e)))?;
    if match_password {
        let msg = format!("New password cannot be the same as the current password");
        return Err(MsgError(msg));
    }

    let pwd = hash_password(new_pwd.to_string())
        .map_err(|e| MsgError(format!("Failed to hash password: {}", e)))?;
    Ok((pwd, match_password))
}

pub async fn update_password(
    user_id: &str,
    new_pwd: &str,
    pool: &Pool<Postgres>,
) -> Result<bool, Error> {
    let mut tx = pool.begin().await?;
    let pwd = new_password(user_id, new_pwd, pool)
        .await
        .map_err(|e| Error::Configuration(e.0.into()))?;

    let sql = "update users set password = $1 where user_id = $2";
    sqlx::query(sql)
        .bind(&pwd.0)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(pwd.1)
}

pub async fn get_users(
    page: i32,
    user_name: &str,
    pool: &Pool<Postgres>,
) -> Result<UserResponse, Error> {
    let mut sql = String::from("select user_name, email from users");
    let offset = if page > 0 { (page - 1) * 10 } else { 0 };
    let users = if !user_name.is_empty() {
        sql.push_str(" where user_name like $1 order by user_name desc limit 10 offset $2");
        let result = sqlx::query(&sql)
            .bind(format!("%{}%", user_name))
            .bind(offset)
            .map(|data: PgRow| User {
                user_name: data.get("user_name"),
                email: data.get("email"),
            })
            .fetch_all(pool)
            .await?;

        result
    } else {
        sql.push_str(" order by user_name desc limit 10 offset $1");
        let result = sqlx::query(&sql)
            .bind(offset)
            .map(|data: PgRow| User {
                user_name: data.get("user_name"),
                email: data.get("email"),
            })
            .fetch_all(pool)
            .await?;
        result
    };
    Ok(UserResponse { page, data: users })
}

#[cfg(test)]
mod tests_user {
    use crate::auth::user::{NewUser, add, get_by_user_name, get_users, update_password};
    use crate::auth::util::hash_password;
    use crate::config::connection;

    use sqlx::Error;

    #[tokio::test]
    async fn test_add_user() -> Result<(), Error> {
        let pool = connection::pg_test().await?;
        let password = "12345".to_string();
        let hash_password = hash_password(password).unwrap();
        let new_user = NewUser::new(
            "Jordan".to_string(),
            "jordan@mail.com".to_string(),
            hash_password.to_string(),
        );
        add(&pool, new_user).await?;

        pool.close().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_add_user_duplicate_user_name() -> Result<(), Error> {
        let pool = connection::pg_test().await?;
        let password = "12345".to_string();
        let hash_password = hash_password(password).unwrap();
        let new_user = NewUser::new(
            "Jordan".to_string(),
            "jordan@mail.com".to_string(),
            hash_password.to_string(),
        );
        let result = add(&pool, new_user).await;

        assert!(result.is_err());
        pool.close().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_get_user_name() -> Result<(), Error> {
        let pool = connection::pg_test().await?;
        let name = "Jordan".to_string();
        let user = get_by_user_name(name.clone(), &pool).await?;
        assert_eq!(name, user.user_name);
        pool.close().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_get_user_name_not_found() -> Result<(), Error> {
        let pool = connection::pg_test().await?;
        let name = "test".to_string();
        let user_name = get_by_user_name(name.clone(), &pool).await;
        assert!(user_name.is_err());
        pool.close().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_update_password() -> Result<(), Error> {
        let user_id = "7718d688-efaa-4195-868e-98fd5ffa3bcf";
        let pool = connection::pg_test().await?;
        let password = "123456";

        let result = update_password(user_id, password, &pool).await;
        assert!(result.is_ok());
        pool.close().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_update_password_with_matching_password() -> Result<(), Error> {
        let user_id = "7718d688-efaa-4195-868e-98fd5ffa3bcf";
        let pool = connection::pg_test().await?;
        let password = "123456";

        let result = update_password(user_id, password, &pool).await;
        assert!(result.is_err());
        pool.close().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_get_users() -> Result<(), Error> {
        let pool = connection::pg_test().await?;
        let result = get_users(0, "", &pool).await;
        assert!(result.is_ok());
        pool.close().await;
        Ok(())
    }

    #[tokio::test]
    async fn test_get_users_with_name() -> Result<(), Error> {
        let pool = connection::pg_test().await?;
        let result = get_users(0, "J", &pool).await;
        assert!(result.is_ok());
        pool.close().await;
        Ok(())
    }
}
