use config::{Config, ConfigError, File, FileFormat};
use sqlx::{Error, Pool, Postgres, postgres::PgPoolOptions};
use std::{result::Result::Ok, time::Duration};

use crate::config::logger::{LogMsg, Logger};

#[derive(Debug)]
pub struct DB {
    pub url: String,
    pub user: String,
    pub password: String,
    pub host: String,
    pub port: i64,
    pub name: String,
    pub max_connection: i64,
    pub min_connection: i64,
    pub acquired_timout: i64,
    pub idle_timout: i64,
}

impl DB {
    pub fn new(
        url: String,
        user: String,
        password: String,
        host: String,
        port: i64,
        name: String,
        max_connection: i64,
        min_connection: i64,
        acquired_timout: i64,
        idle_timout: i64,
    ) -> DB {
        DB {
            url: url,
            user: user,
            password: password,
            host: host,
            name: name,
            port: port,
            max_connection: max_connection,
            min_connection: min_connection,
            acquired_timout: acquired_timout,
            idle_timout: idle_timout,
        }
    }
}

#[derive(Debug)]
pub struct ConnectionBuilder;

impl ConnectionBuilder {
    pub fn build(name: &str) -> Result<Config, ConfigError> {
        let builder = Config::builder()
            .add_source(File::new(name, FileFormat::Toml))
            .build();

        match builder {
            Ok(build) => Ok(build),
            Err(error) => {
                Logger::init();
                let log = Logger;
                let msg = format!("Failed to execute environment : {:?}", error);
                log.err(&msg);
                panic!("Failed to execute environment : {:?}", error)
            }
        }
    }
}

pub trait Connection {
    async fn pool(&self) -> Result<Pool<Postgres>, Error>;
}

impl Connection for DB {
    async fn pool(&self) -> Result<Pool<Postgres>, Error> {
        let url = self.url.clone();
        let result = PgPoolOptions::new()
            .max_connections(self.max_connection as u32)
            .min_connections(self.min_connection as u32)
            .acquire_timeout(Duration::from_secs(self.acquired_timout as u64))
            .idle_timeout(Duration::from_secs(self.idle_timout as u64))
            .connect(&url)
            .await;

        match result {
            Ok(v) => Ok(v),
            Err(e) => {
                Logger::init();
                let log = Logger;
                let msg = format!("Failed to connect into database : {:?}", e);
                log.err(&msg);
                panic!("Failed to connect into database : {}", e)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::config::connection::{Connection, ConnectionBuilder, DB};
    use sqlx::Error;

    #[test]
    fn test_environment() {
        let con = ConnectionBuilder::build("dev.toml").unwrap();

        let db: DB = DB {
            url: con.get_string("database.url").unwrap(),
            user: con.get_string("database.user").unwrap(),
            name: con.get_string("database.name").unwrap(),
            host: con.get_string("database.host").unwrap(),
            port: con.get_int("database.port").unwrap(),
            password: con.get_string("database.password").unwrap(),
            max_connection: con.get_int("database.max_connection").unwrap(),
            min_connection: con.get_int("database.min_connection").unwrap(),
            acquired_timout: con.get_int("database.acquire_timeout").unwrap(),
            idle_timout: con.get_int("database.idle_timeout").unwrap(),
        };

        assert_eq!(db.user, "postgres");
        assert_eq!(db.name, "roger_dev");
        assert_eq!(db.host, "localhost");
        assert_eq!(db.port, 5432);
        assert_eq!(db.max_connection, 10);
        assert_eq!(db.min_connection, 5);
        assert_eq!(db.acquired_timout, 5);
        assert_eq!(db.idle_timout, 60);
    }

    #[test]
    #[should_panic(expected = "Failed to execute environment ")]
    fn test_environment_error() {
        let _ = ConnectionBuilder::build("d.toml");
    }

    #[tokio::test]
    async fn test_pool_connection() -> Result<(), Error> {
        let con = ConnectionBuilder::build("dev.toml").unwrap();

        let db: DB = DB {
            url: con.get_string("database.url").unwrap(),
            user: con.get_string("database.user").unwrap(),
            name: con.get_string("database.name").unwrap(),
            host: con.get_string("database.host").unwrap(),
            port: con.get_int("database.port").unwrap(),
            password: con.get_string("database.password").unwrap(),
            max_connection: con.get_int("database.max_connection").unwrap(),
            min_connection: con.get_int("database.min_connection").unwrap(),
            acquired_timout: con.get_int("database.acquire_timeout").unwrap(),
            idle_timout: con.get_int("database.idle_timeout").unwrap(),
        };
        let pool = Connection::pool(&db).await?;
        pool.close().await;
        Ok(())
    }

    #[tokio::test]
    #[should_panic(expected = "invalid identifier")]
    async fn test_pool_connection_failed() {
        let con = ConnectionBuilder::build("dev.toml").unwrap();

        let db: DB = DB {
            url: con.get_string("").unwrap(),
            user: "".to_string(),
            name: "".to_string(),
            host: "".to_string(),
            port: 0,
            password: "".to_string(),
            max_connection: 0,
            min_connection: 0,
            acquired_timout: 0,
            idle_timout: 0,
        };
        let _ = Connection::pool(&db).await;
    }
}
