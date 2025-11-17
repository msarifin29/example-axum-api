use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use sqlx::{Error, Pool, Postgres, Row, postgres::PgRow};
use uuid::Uuid;
use validator::{Validate, ValidationError};

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

#[cfg(test)]
mod tests_user {
    use crate::auth::user::{NewUser, add, get_by_user_name};
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
}
