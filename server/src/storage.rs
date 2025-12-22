//! Simple storage layer for user data.
//! Works with both SQLite and PostgreSQL.

use rust_web_common::db::DatabasePool;
use serde_json::Value;
use uuid::Uuid;

/// Stores user data as JSON/JSONB in the database.
pub async fn store_user_data(
    db: &DatabasePool,
    user_id: &Uuid,
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
    user_id: &Uuid,
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
            let value: Value = serde_json::from_str(&data_str).map_err(|e| {
                sqlx::Error::Protocol(format!("JSON deserialization error: {}", e))
            })?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

/// Gets a specific key from user data.
#[allow(dead_code)]
pub async fn get_user_data_key(
    db: &DatabasePool,
    user_id: &Uuid,
    key: &str,
) -> Result<Option<Value>, sqlx::Error> {
    let data = load_user_data(db, user_id).await?;

    Ok(data.and_then(|d| d.get(key).cloned()))
}

/// Sets a specific key in user data.
pub async fn set_user_data_key(
    db: &DatabasePool,
    user_id: &Uuid,
    key: &str,
    value: &Value,
) -> Result<(), sqlx::Error> {
    let mut data = load_user_data(db, user_id)
        .await?
        .unwrap_or_else(|| serde_json::json!({}));

    data.as_object_mut()
        .expect("data should be an object")
        .insert(key.to_string(), value.clone());

    store_user_data(db, user_id, &data).await
}
