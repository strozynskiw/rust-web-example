//! Authentication and authorization module with security best practices
//!
//! Features:
//! - Secure password hashing with Argon2id
//! - Session-based authentication
//! - Role-based access control (RBAC)
//! - Protection against timing attacks

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};
use std::fmt;
use uuid::Uuid;

use web_template_common::db::DatabasePool;

pub type SharedClient = DatabasePool;

/// User role enum for role-based access control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    User,
    Admin,
}

impl UserRole {
    /// Convert role to database string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::User => "user",
            UserRole::Admin => "admin",
        }
    }

    /// Parse role from database string
    #[allow(dead_code)]
    pub fn from_str(s: &str) -> Result<Self, AuthError> {
        match s.to_lowercase().as_str() {
            "user" => Ok(UserRole::User),
            "admin" => Ok(UserRole::Admin),
            _ => Err(AuthError::InvalidRole),
        }
    }

    /// Check if role has admin privileges
    #[allow(dead_code)]
    pub fn is_admin(&self) -> bool {
        matches!(self, UserRole::Admin)
    }
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// User model from database
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    #[allow(dead_code)] // Used for authentication
    pub password_hash: String,
    pub role: String,
    pub is_active: bool,
    #[serde(skip_serializing)]
    pub created_at: chrono::NaiveDateTime,
    #[serde(skip_serializing)]
    #[allow(dead_code)] // Used in templates
    pub updated_at: chrono::NaiveDateTime,
    #[serde(skip_serializing)]
    pub last_login_at: Option<chrono::NaiveDateTime>,
}

impl User {
    /// Get user's role as enum
    #[allow(dead_code)]
    pub fn get_role(&self) -> Result<UserRole, AuthError> {
        UserRole::from_str(&self.role)
    }

    /// Check if user is admin
    pub fn is_admin(&self) -> bool {
        self.get_role().map(|r| r.is_admin()).unwrap_or(false)
    }

    /// Get user ID as UUID
    #[allow(dead_code)]
    pub fn uuid(&self) -> Result<Uuid, AuthError> {
        Uuid::parse_str(&self.id).map_err(|_| AuthError::InvalidUserId)
    }
}

/// Session model from database
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub expires_at: chrono::NaiveDateTime,
    pub created_at: chrono::NaiveDateTime,
}

impl Session {
    /// Check if session is expired
    #[allow(dead_code)]
    pub fn is_expired(&self) -> bool {
        self.expires_at < chrono::Utc::now().naive_utc()
    }

    /// Get session ID as UUID
    #[allow(dead_code)]
    pub fn uuid(&self) -> Result<Uuid, AuthError> {
        Uuid::parse_str(&self.id).map_err(|_| AuthError::InvalidSessionId)
    }
}

/// Authentication errors
#[derive(Debug)]
#[allow(dead_code)]
pub enum AuthError {
    InvalidCredentials,
    UserNotFound,
    SessionNotFound,
    UserInactive,
    InvalidRole,
    InvalidUserId,
    InvalidSessionId,
    Unauthorized,
    PasswordHashError(String),
    DatabaseError(sqlx::Error),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::InvalidCredentials => write!(f, "Invalid credentials"),
            AuthError::UserNotFound => write!(f, "User not found"),
            AuthError::SessionNotFound => write!(f, "Session not found or expired"),
            AuthError::UserInactive => write!(f, "User account is inactive"),
            AuthError::InvalidRole => write!(f, "Invalid role"),
            AuthError::InvalidUserId => write!(f, "Invalid user ID"),
            AuthError::InvalidSessionId => write!(f, "Invalid session ID"),
            AuthError::Unauthorized => write!(f, "Unauthorized access"),
            AuthError::PasswordHashError(msg) => write!(f, "Password hashing error: {}", msg),
            AuthError::DatabaseError(e) => write!(f, "Database error: {}", e),
        }
    }
}

impl std::error::Error for AuthError {}

impl From<sqlx::Error> for AuthError {
    fn from(err: sqlx::Error) -> Self {
        AuthError::DatabaseError(err)
    }
}

/// Password hashing service using Argon2id (most secure variant)
pub struct PasswordService;

impl PasswordService {
    /// Hash a password securely using Argon2id
    ///
    /// Uses recommended parameters:
    /// - Argon2id (hybrid mode resistant to both GPU and side-channel attacks)
    /// - Random salt per password
    /// - Standard parameters suitable for authentication
    pub fn hash_password(password: &str) -> Result<String, AuthError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| AuthError::PasswordHashError(e.to_string()))
    }

    /// Verify a password against a hash (constant-time comparison)
    #[allow(dead_code)]
    pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| AuthError::PasswordHashError(e.to_string()))?;

        let argon2 = Argon2::default();

        // This performs constant-time comparison to prevent timing attacks
        Ok(argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

/// Session duration in hours
#[allow(dead_code)]
const SESSION_DURATION_HOURS: i64 = 24 * 7; // 7 days

/// Authentication service for user and session management
pub struct AuthService;

impl AuthService {
    /// Create a new user with hashed password
    #[allow(dead_code)]
    pub async fn create_user(
        db: &SharedClient,
        username: &str,
        email: &str,
        password: &str,
        role: UserRole,
    ) -> Result<User, AuthError> {
        let password_hash = PasswordService::hash_password(password)?;
        let id = Uuid::new_v4().to_string();
        let role_str = role.as_str();

        match db {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO users (id, username, email, password_hash, role, is_active)
                    VALUES ($1, $2, $3, $4, $5, TRUE)
                    "#,
                )
                .bind(&id)
                .bind(username)
                .bind(email)
                .bind(&password_hash)
                .bind(role_str)
                .execute(pool)
                .await?;

                Self::get_user_by_id(db, &id).await
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO users (id, username, email, password_hash, role, is_active)
                    VALUES ($1, $2, $3, $4, $5, TRUE)
                    "#,
                )
                .bind(&id)
                .bind(username)
                .bind(email)
                .bind(&password_hash)
                .bind(role_str)
                .execute(pool)
                .await?;

                Self::get_user_by_id(db, &id).await
            }
        }
    }

    /// Authenticate user with username and password
    pub async fn authenticate(
        db: &SharedClient,
        username: &str,
        password: &str,
    ) -> Result<User, AuthError> {
        // Get user with password hash
        let (user, password_hash) = match db {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    SELECT id, username, email, password_hash, role, is_active, 
                           created_at, updated_at, last_login_at
                    FROM users
                    WHERE username = $1
                    "#,
                )
                .bind(username)
                .fetch_optional(pool)
                .await?
                .map(|row| {
                    let user = User {
                        id: row.get("id"),
                        username: row.get("username"),
                        email: row.get("email"),
                        password_hash: String::new(),
                        role: row.get("role"),
                        is_active: row.get("is_active"),
                        created_at: row.get("created_at"),
                        updated_at: row.get("updated_at"),
                        last_login_at: row.get("last_login_at"),
                    };
                    let hash: String = row.get("password_hash");
                    (user, hash)
                })
                .ok_or(AuthError::UserNotFound)?
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    SELECT id, username, email, password_hash, role, is_active, 
                           created_at, updated_at, last_login_at
                    FROM users
                    WHERE username = $1
                    "#,
                )
                .bind(username)
                .fetch_optional(pool)
                .await?
                .map(|row| {
                    let user = User {
                        id: row.get("id"),
                        username: row.get("username"),
                        email: row.get("email"),
                        password_hash: String::new(),
                        role: row.get("role"),
                        is_active: row.get("is_active"),
                        created_at: row.get("created_at"),
                        updated_at: row.get("updated_at"),
                        last_login_at: row.get("last_login_at"),
                    };
                    let hash: String = row.get("password_hash");
                    (user, hash)
                })
                .ok_or(AuthError::UserNotFound)?
            }
        };

        // Check if user is active
        if !user.is_active {
            return Err(AuthError::UserInactive);
        }

        // Verify password (constant-time comparison)
        if !PasswordService::verify_password(password, &password_hash)? {
            return Err(AuthError::InvalidCredentials);
        }

        // Update last login time
        Self::update_last_login(db, &user.id).await?;

        Ok(user)
    }

    /// Create a new session for a user
    pub async fn create_session(db: &SharedClient, user_id: &str) -> Result<Session, AuthError> {
        let session_id = Uuid::new_v4().to_string();
        let expires_at = chrono::Utc::now().naive_utc()
            + chrono::Duration::hours(SESSION_DURATION_HOURS);

        match db {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO sessions (id, user_id, expires_at)
                    VALUES ($1, $2, $3)
                    "#,
                )
                .bind(&session_id)
                .bind(user_id)
                .bind(expires_at)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    r#"
                    INSERT INTO sessions (id, user_id, expires_at)
                    VALUES ($1, $2, $3)
                    "#,
                )
                .bind(&session_id)
                .bind(user_id)
                .bind(expires_at)
                .execute(pool)
                .await?;
            }
        }

        Ok(Session {
            id: session_id,
            user_id: user_id.to_string(),
            expires_at,
            created_at: chrono::Utc::now().naive_utc(),
        })
    }

    /// Validate a session and return the user
    pub async fn validate_session(db: &SharedClient, session_id: &str) -> Result<User, AuthError> {
        let session = Self::get_session(db, session_id).await?;

        if session.is_expired() {
            Self::delete_session(db, session_id).await?;
            return Err(AuthError::SessionNotFound);
        }

        Self::get_user_by_id(db, &session.user_id).await
    }

    /// Get session by ID
    async fn get_session(db: &SharedClient, session_id: &str) -> Result<Session, AuthError> {
        match db {
            DatabasePool::Sqlite(pool) => sqlx::query_as::<_, Session>(
                r#"
                SELECT id, user_id, expires_at, created_at
                FROM sessions
                WHERE id = $1
                "#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await?
            .ok_or(AuthError::SessionNotFound),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, Session>(
                r#"
                SELECT id, user_id, expires_at, created_at
                FROM sessions
                WHERE id = $1
                "#,
            )
            .bind(session_id)
            .fetch_optional(pool)
            .await?
            .ok_or(AuthError::SessionNotFound),
        }
    }

    /// Delete a session (logout)
    pub async fn delete_session(db: &SharedClient, session_id: &str) -> Result<(), AuthError> {
        match db {
            DatabasePool::Sqlite(pool) => {
                sqlx::query("DELETE FROM sessions WHERE id = $1")
                    .bind(session_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM sessions WHERE id = $1")
                    .bind(session_id)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }

    /// Delete all sessions for a user (logout from all devices)
    #[allow(dead_code)]
    pub async fn delete_all_user_sessions(
        db: &SharedClient,
        user_id: &str,
    ) -> Result<(), AuthError> {
        match db {
            DatabasePool::Sqlite(pool) => {
                sqlx::query("DELETE FROM sessions WHERE user_id = $1")
                    .bind(user_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM sessions WHERE user_id = $1")
                    .bind(user_id)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }

    /// Clean up expired sessions (should be run periodically)
    #[allow(dead_code)]
    pub async fn cleanup_expired_sessions(db: &SharedClient) -> Result<u64, AuthError> {
        let now = chrono::Utc::now().naive_utc();

        let rows_affected = match db {
            DatabasePool::Sqlite(pool) => {
                sqlx::query("DELETE FROM sessions WHERE expires_at < $1")
                    .bind(now)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM sessions WHERE expires_at < $1")
                    .bind(now)
                    .execute(pool)
                    .await?
                    .rows_affected()
            }
        };

        Ok(rows_affected)
    }

    /// Get user by ID
    pub async fn get_user_by_id(db: &SharedClient, user_id: &str) -> Result<User, AuthError> {
        match db {
            DatabasePool::Sqlite(pool) => sqlx::query_as::<_, User>(
                r#"
                SELECT id, username, email, '' as password_hash, role, is_active, 
                       created_at, updated_at, last_login_at
                FROM users
                WHERE id = $1
                "#,
            )
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .ok_or(AuthError::UserNotFound),
            DatabasePool::Postgres(pool) => sqlx::query_as::<_, User>(
                r#"
                SELECT id, username, email, '' as password_hash, role, is_active, 
                       created_at, updated_at, last_login_at
                FROM users
                WHERE id = $1
                "#,
            )
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .ok_or(AuthError::UserNotFound),
        }
    }

    /// Update user's last login time
    async fn update_last_login(db: &SharedClient, user_id: &str) -> Result<(), AuthError> {
        let now = chrono::Utc::now().naive_utc();

        match db {
            DatabasePool::Sqlite(pool) => {
                sqlx::query("UPDATE users SET last_login_at = $1 WHERE id = $2")
                    .bind(now)
                    .bind(user_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("UPDATE users SET last_login_at = $1 WHERE id = $2")
                    .bind(now)
                    .bind(user_id)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }

    /// List all users (admin only)
    pub async fn list_users(db: &SharedClient) -> Result<Vec<User>, AuthError> {
        match db {
            DatabasePool::Sqlite(pool) => Ok(sqlx::query_as::<_, User>(
                r#"
                SELECT id, username, email, '' as password_hash, role, is_active, 
                       created_at, updated_at, last_login_at
                FROM users
                ORDER BY created_at DESC
                "#,
            )
            .fetch_all(pool)
            .await?),
            DatabasePool::Postgres(pool) => Ok(sqlx::query_as::<_, User>(
                r#"
                SELECT id, username, email, '' as password_hash, role, is_active, 
                       created_at, updated_at, last_login_at
                FROM users
                ORDER BY created_at DESC
                "#,
            )
            .fetch_all(pool)
            .await?),
        }
    }

    /// Update user role (admin only)
    pub async fn update_user_role(
        db: &SharedClient,
        user_id: &str,
        new_role: UserRole,
    ) -> Result<(), AuthError> {
        let role_str = new_role.as_str();

        match db {
            DatabasePool::Sqlite(pool) => {
                sqlx::query("UPDATE users SET role = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                    .bind(role_str)
                    .bind(user_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query("UPDATE users SET role = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                    .bind(role_str)
                    .bind(user_id)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }

    /// Toggle user active status (admin only)
    pub async fn toggle_user_status(
        db: &SharedClient,
        user_id: &str,
        is_active: bool,
    ) -> Result<(), AuthError> {
        match db {
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "UPDATE users SET is_active = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
                )
                .bind(is_active)
                .bind(user_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "UPDATE users SET is_active = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
                )
                .bind(is_active)
                .bind(user_id)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }
}
