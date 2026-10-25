//! Simple storage layer for user data.
//! Works with both SQLite and PostgreSQL.

use serde_json::Value;
use web_template_common::db::DatabasePool;

/// Replaces a complete JSON document stored as TEXT.
#[allow(dead_code)]
pub async fn store_user_data(
    db: &DatabasePool,
    user_id: &str,
    data: &Value,
) -> Result<(), sqlx::Error> {
    let data_str = serde_json::to_string(data)
        .map_err(|e| sqlx::Error::Protocol(format!("JSON serialization error: {}", e)))?;

    match db {
        DatabasePool::Sqlite(pool) => {
            sqlx::query(
                r#"
                INSERT INTO user_data (user_id, data, updated_at)
                VALUES ($1, $2, CURRENT_TIMESTAMP)
                ON CONFLICT (user_id)
                DO UPDATE SET
                    data = $2,
                    updated_at = CURRENT_TIMESTAMP
                "#,
            )
            .bind(user_id.to_string())
            .bind(&data_str)
            .execute(pool)
            .await?;
        }
        DatabasePool::Postgres(pool) => {
            sqlx::query(
                r#"
                INSERT INTO user_data (user_id, data, updated_at)
                VALUES ($1, $2, CURRENT_TIMESTAMP)
                ON CONFLICT (user_id)
                DO UPDATE SET
                    data = $2,
                    updated_at = CURRENT_TIMESTAMP
                "#,
            )
            .bind(user_id.to_string())
            .bind(&data_str)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

/// Loads user data from the database.
pub async fn load_user_data(
    db: &DatabasePool,
    user_id: &str,
) -> Result<Option<Value>, sqlx::Error> {
    let result: Option<String> = match db {
        DatabasePool::Sqlite(pool) => {
            sqlx::query_scalar("SELECT data FROM user_data WHERE user_id = $1")
                .bind(user_id.to_string())
                .fetch_optional(pool)
                .await?
        }
        DatabasePool::Postgres(pool) => {
            sqlx::query_scalar("SELECT data FROM user_data WHERE user_id = $1")
                .bind(user_id.to_string())
                .fetch_optional(pool)
                .await?
        }
    };

    match result {
        Some(data_str) => {
            let value: Value = serde_json::from_str(&data_str)
                .map_err(|e| sqlx::Error::Protocol(format!("JSON deserialization error: {}", e)))?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

/// Gets a specific key from user data.
#[allow(dead_code)]
pub async fn get_user_data_key(
    db: &DatabasePool,
    user_id: &str,
    key: &str,
) -> Result<Option<Value>, sqlx::Error> {
    let data = load_user_data(db, user_id).await?;

    Ok(data.and_then(|d| d.get(key).cloned()))
}

/// Sets a specific key in user data.
pub async fn set_user_data_key(
    db: &DatabasePool,
    user_id: &str,
    key: &str,
    value: &Value,
) -> Result<(), sqlx::Error> {
    // Optimistic compare-and-swap: a competing writer causes a fresh read,
    // never a successful response that silently overwrites another key.
    for _ in 0..32 {
        let previous: Option<String> = match db {
            DatabasePool::Sqlite(pool) => {
                sqlx::query_scalar("SELECT data FROM user_data WHERE user_id = $1")
                    .bind(user_id)
                    .fetch_optional(pool)
                    .await?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query_scalar("SELECT data FROM user_data WHERE user_id = $1")
                    .bind(user_id)
                    .fetch_optional(pool)
                    .await?
            }
        };
        let mut data = match &previous {
            Some(raw) => serde_json::from_str::<Value>(raw)
                .map_err(|e| sqlx::Error::Protocol(e.to_string()))?,
            None => serde_json::json!({}),
        };
        let object = data.as_object_mut().ok_or_else(|| {
            sqlx::Error::Protocol("Stored user data must be a JSON object".into())
        })?;
        object.insert(key.to_owned(), value.clone());
        let serialized = data.to_string();
        let sql =
            "INSERT INTO user_data (user_id, data, updated_at) VALUES ($1, $2, CURRENT_TIMESTAMP)
            ON CONFLICT (user_id) DO UPDATE SET data = $2, updated_at = CURRENT_TIMESTAMP
            WHERE user_data.data = $3";
        let changed = match db {
            DatabasePool::Sqlite(pool) => sqlx::query(sql)
                .bind(user_id)
                .bind(&serialized)
                .bind(&previous)
                .execute(pool)
                .await?
                .rows_affected(),
            DatabasePool::Postgres(pool) => sqlx::query(sql)
                .bind(user_id)
                .bind(&serialized)
                .bind(&previous)
                .execute(pool)
                .await?
                .rows_affected(),
        };
        if changed == 1 {
            return Ok(());
        }
        tokio::task::yield_now().await;
    }
    Err(sqlx::Error::Protocol(
        "Concurrent update contention; retry the request".into(),
    ))
}
