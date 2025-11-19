use config::{Config, ConfigError, File, FileFormat};
use sqlx::{Error, Pool, Postgres, postgres::PgPoolOptions};
use std::{result::Result::Ok, time::Duration};

use crate::{
    auth::util::MsgError,
    config::logger::{LogMsg, Logger},
};

#[derive(Debug)]
pub struct DB {
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

#[derive(Debug)]
pub struct TCP {
    pub ip: String,
    pub port: i32,
}

#[derive(Debug)]
pub struct Configure;

impl Configure {
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

#[derive(Debug)]
pub struct ConnectionBuilder(pub String);

impl ConnectionBuilder {
    pub async fn new(&self) -> Result<Pool<Postgres>, Error> {
        let con = Configure::build(&self.0).unwrap();

        let db: DB = DB {
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

        let url = format!(
            "postgres://{}:{}@{}:{}/{}",
            db.user, db.password, db.host, db.port, db.name
        );
        let result = PgPoolOptions::new()
            .max_connections(db.max_connection as u32)
            .min_connections(db.min_connection as u32)
            .acquire_timeout(Duration::from_secs(db.acquired_timout as u64))
            .idle_timeout(Duration::from_secs(db.idle_timout as u64))
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

    pub fn listen_on(&self) -> Result<TCP, MsgError> {
        let result = Configure::build(&self.0);
        match result {
            Ok(con) => Ok(TCP {
                ip: con.get_string("tcp.ip").unwrap(),
                port: con.get_int("tcp.port").unwrap() as i32,
            }),
            Err(e) => {
                Logger::init();
                let log = Logger;
                let msg = format!("Failed to execute environment : {:?}", e);
                log.err(&msg);
                panic!("Failed to execute environment : {:?}", e)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::config::connection::{Configure, ConnectionBuilder, DB};
    use sqlx::Error;

    #[test]
    fn test_environment() {
        let con = Configure::build("dev.toml").unwrap();

        let db: DB = DB {
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
    #[should_panic(expected = "Failed to execute environment : configuration file")]
    fn test_environment_error() {
        let _ = Configure::build("d.toml");
    }

    #[tokio::test]
    async fn test_pool_connection() -> Result<(), Error> {
        let builder = ConnectionBuilder(String::from("dev.toml"));
        let pool = ConnectionBuilder::new(&builder).await?;

        pool.close().await;
        Ok(())
    }

    #[tokio::test]
    #[should_panic(expected = "Failed to execute environment : configuration file")]
    async fn test_pool_connection_failed() {
        let builder = ConnectionBuilder(String::from("de.toml"));
        let _ = ConnectionBuilder::new(&builder).await;
    }

    #[tokio::test]
    async fn test_pool_connection_helper() -> Result<(), Error> {
        let builder = ConnectionBuilder(String::from("dev.toml"));
        let pool = ConnectionBuilder::new(&builder).await?;
        pool.close().await;
        Ok(())
    }
}
