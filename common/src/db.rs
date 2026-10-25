use sqlx::{postgres::PgPool, sqlite::SqlitePool};
use std::time::Duration;

const DB_MAX_CONNECTIONS: u32 = 10;
const DB_MIN_CONNECTIONS: u32 = 1;
const DB_TIMEOUT_SECS: u64 = 10;

/// Database pool type that can be either SQLite or PostgreSQL
#[derive(Clone)]
pub enum DatabasePool {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

/// Creates a database connection pool for either SQLite or PostgreSQL.
///
/// The database type is automatically detected from the connection string:
/// - `sqlite://` or `sqlite:` -> SQLite
/// - `postgresql://` or `postgres://` -> PostgreSQL
pub async fn create_pool(database_url: &str) -> anyhow::Result<DatabasePool> {
    if database_url.starts_with("sqlite://") || database_url.starts_with("sqlite:") {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        // Parse the complete URL (including mode/query options). SQLx otherwise
        // refuses a missing file, which breaks the documented first run.
        let options = database_url
            .parse::<SqliteConnectOptions>()?
            .create_if_missing(true)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(DB_TIMEOUT_SECS));
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(DB_TIMEOUT_SECS))
            .connect_with(options)
            .await?;

        Ok(DatabasePool::Sqlite(pool))
    } else if database_url.starts_with("postgresql://") || database_url.starts_with("postgres://") {
        // PostgreSQL connection
        use sqlx::postgres::PgPoolOptions;
        let pool = PgPoolOptions::new()
            .max_connections(DB_MAX_CONNECTIONS)
            .min_connections(DB_MIN_CONNECTIONS)
            .acquire_timeout(Duration::from_secs(DB_TIMEOUT_SECS))
            .idle_timeout(Duration::from_secs(DB_TIMEOUT_SECS))
            .max_lifetime(Duration::from_secs(DB_TIMEOUT_SECS))
            .connect(database_url)
            .await?;

        Ok(DatabasePool::Postgres(pool))
    } else {
        anyhow::bail!("Unsupported database URL format. Use 'sqlite://' or 'postgresql://'");
    }
}
