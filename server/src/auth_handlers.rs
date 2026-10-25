//! Authentication route handlers (login, logout, register)

use axum::{
    Form,
    extract::Extension,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;
use tera::Context;
use tower_cookies::{Cookie, Cookies};

use crate::auth::{AuthService, User, UserRole};
use crate::error::HttpError;
use crate::middlewares::SESSION_COOKIE;
use crate::security::SecurityConfig;
use crate::{SharedClient, SharedTera};

/// Session duration in seconds (7 days)
const SESSION_MAX_AGE: i64 = 60 * 60 * 24 * 7;

/// Login page handler
pub async fn login_page(
    Extension(tera): Extension<SharedTera>,
    Extension(user): Extension<Option<User>>,
) -> Result<Html<String>, HttpError> {
    if let Some(user) = user {
        return Err(Redirect::to(if user.is_admin() {
            "/admin"
        } else {
            "/profile"
        })
        .into_response()
        .into());
    }

    let context = Context::new();
    let html = tera
        .read()
        .await
        .render("auth/login.html", &context)
        .map_err(|e| {
            tracing::error!("Template error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to render template",
            )
                .into_response()
        })?;

    Ok(Html(html))
}

/// Login form data
#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

/// Login handler
pub async fn login(
    Extension(db): Extension<SharedClient>,
    Extension(config): Extension<SecurityConfig>,
    cookies: Cookies,
    Form(form): Form<LoginForm>,
) -> Result<Response, HttpError> {
    // Authenticate user
    let user = AuthService::authenticate(&db, &form.username, &form.password)
        .await
        .map_err(|e| {
            if matches!(e, crate::auth::AuthError::Busy) {
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    [("retry-after", "1")],
                    "Authentication busy; retry shortly",
                )
                    .into_response();
            }
            if matches!(
                e,
                crate::auth::AuthError::DatabaseError(_)
                    | crate::auth::AuthError::PasswordHashError(_)
            ) {
                tracing::error!(error = %e, "Authentication unavailable");
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Authentication unavailable",
                )
                    .into_response();
            }
            (StatusCode::UNAUTHORIZED, "Invalid username or password").into_response()
        })?;

    // Create session
    let session = AuthService::create_session(&db, &user.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create session: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create session",
            )
                .into_response()
        })?;

    // Set session cookie
    let cookie = Cookie::build((SESSION_COOKIE, session.id))
        .path("/")
        .max_age(time::Duration::seconds(SESSION_MAX_AGE))
        .http_only(true)
        .secure(config.secure_cookies)
        .same_site(tower_cookies::cookie::SameSite::Lax)
        .build();

    cookies.add(cookie);

    tracing::info!("User '{}' logged in successfully", user.username);

    // Redirect based on role
    if user.is_admin() {
        Ok(Redirect::to("/admin").into_response())
    } else {
        Ok(Redirect::to("/profile").into_response())
    }
}

/// Logout handler
pub async fn logout(
    Extension(db): Extension<SharedClient>,
    cookies: Cookies,
) -> Result<Response, HttpError> {
    // Get session ID from cookie
    if let Some(session_cookie) = cookies.get(SESSION_COOKIE) {
        let session_id = session_cookie.value();

        // Delete session from database
        if let Err(e) = AuthService::delete_session(&db, session_id).await {
            tracing::error!("Failed to delete session: {}", e);
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                "Logout failed; please retry",
            )
                .into_response()
                .into());
        }
    }

    // Remove session cookie
    cookies.remove(Cookie::build(SESSION_COOKIE).path("/").build());

    tracing::info!("User logged out");

    Ok(Redirect::to("/login").into_response())
}

/// Register page handler
pub async fn register_page(
    Extension(tera): Extension<SharedTera>,
    Extension(user): Extension<Option<User>>,
) -> Result<Html<String>, HttpError> {
    // If already logged in, redirect to home
    if user.is_some() {
        return Err(Redirect::to("/profile").into_response().into());
    }

    let context = Context::new();
    let html = tera
        .read()
        .await
        .render("auth/register.html", &context)
        .map_err(|e| {
            tracing::error!("Template error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to render template",
            )
                .into_response()
        })?;

    Ok(Html(html))
}

/// Register form data
#[derive(Deserialize)]
pub struct RegisterForm {
    username: String,
    email: String,
    password: String,
    password_confirm: String,
}

/// Register handler
pub async fn register(
    Extension(db): Extension<SharedClient>,
    Extension(config): Extension<SecurityConfig>,
    cookies: Cookies,
    Form(form): Form<RegisterForm>,
) -> Result<Response, HttpError> {
    // Validate passwords match
    if form.password != form.password_confirm {
        return Err((StatusCode::BAD_REQUEST, "Passwords do not match")
            .into_response()
            .into());
    }

    crate::auth::validate_registration(&form.username, &form.email, &form.password)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()).into_response())?;

    // Create user (default role is 'user')
    let user = AuthService::create_user(
        &db,
        &form.username,
        &form.email,
        &form.password,
        UserRole::User,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to create user: {}", e);
        match e {
            crate::auth::AuthError::Busy => (
                StatusCode::TOO_MANY_REQUESTS,
                [("retry-after", "1")],
                "Authentication busy; retry shortly",
            )
                .into_response(),
            crate::auth::AuthError::DatabaseError(sqlx::Error::Database(db_err)) => {
                if db_err.is_unique_violation() {
                    (StatusCode::CONFLICT, "Username or email already exists").into_response()
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create user").into_response()
                }
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create user").into_response(),
        }
    })?;

    // Create session
    let session = AuthService::create_session(&db, &user.id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create session: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create session",
            )
                .into_response()
        })?;

    // Set session cookie
    let cookie = Cookie::build((SESSION_COOKIE, session.id))
        .path("/")
        .max_age(time::Duration::seconds(SESSION_MAX_AGE))
        .http_only(true)
        .secure(config.secure_cookies)
        .same_site(tower_cookies::cookie::SameSite::Lax)
        .build();

    cookies.add(cookie);

    tracing::info!("User '{}' registered successfully", user.username);

    Ok(Redirect::to("/").into_response())
}
