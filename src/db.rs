//! Database connection helpers.
//!
//! This module provides a small wrapper around the Diesel connection pool and
//! utilities to establish a connection to the SQLite database used in the
//! application.

use std::time::Duration;

use diesel::connection::SimpleConnection;
use diesel::r2d2::{ConnectionManager, CustomizeConnection, Pool, PoolError, PooledConnection};
use diesel::sqlite::SqliteConnection;
use log::error;

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;
pub type DbConnection = PooledConnection<ConnectionManager<SqliteConnection>>;

#[derive(Debug)]
/// Options that are applied each time a connection is acquired from the pool.
pub struct ConnectionOptions {
    /// Enable Write Ahead Logging mode for SQLite.
    pub enable_wal: bool,
    /// Enforce foreign key checks for SQLite.
    pub enable_foreign_keys: bool,
    /// Timeout to wait for a locked database.
    pub busy_timeout: Option<Duration>,
}

impl CustomizeConnection<SqliteConnection, diesel::r2d2::Error> for ConnectionOptions {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        (|| {
            if self.enable_wal {
                conn.batch_execute("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
            }
            if self.enable_foreign_keys {
                conn.batch_execute("PRAGMA foreign_keys = ON;")?;
            }
            if let Some(d) = self.busy_timeout {
                conn.batch_execute(&format!("PRAGMA busy_timeout = {};", d.as_millis()))?;
            }
            Ok(())
        })()
        .map_err(diesel::r2d2::Error::QueryError)
    }
}

/// Create a Diesel connection pool for the given database URL.
pub fn establish_connection_pool(database_url: &str) -> Result<DbPool, PoolError> {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    Pool::builder()
        .connection_customizer(Box::new(ConnectionOptions {
            enable_wal: true,
            enable_foreign_keys: true,
            busy_timeout: Some(Duration::from_secs(30)),
        }))
        .build(manager)
}

/// Retrieve a connection from the pool
pub fn get_connection(pool: &DbPool) -> Result<DbConnection, PoolError> {
    match pool.get() {
        Ok(conn) => Ok(conn),
        Err(e) => {
            error!("Failed to get connection from pool: {e}");
            Err(e)
        }
    }
}
