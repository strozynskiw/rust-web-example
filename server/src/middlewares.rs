//! Authentication and authorization middleware

use axum::{
    extract::{Extension, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use tower_cookies::Cookies;

use crate::SharedClient;
use crate::auth::{AuthError, AuthService, User};
use crate::error::HttpError;

/// Cookie name for session ID
pub const SESSION_COOKIE: &str = "session_id";

/// Middleware that requires authentication
///
/// Validates the session and injects the authenticated user into request extensions.
/// Redirects to login page if not authenticated.
pub async fn require_auth(
    Extension(db): Extension<SharedClient>,
    cookies: Cookies,
    mut request: Request,
    next: Next,
) -> Result<Response, HttpError> {
    let session_id = cookies
        .get(SESSION_COOKIE)
        .map(|cookie| cookie.value().to_string());

    if let Some(session_id) = session_id {
        match AuthService::validate_session(&db, &session_id).await {
            Ok(user) => {
                // Inject authenticated user into request
                request.extensions_mut().insert(user);
                return Ok(next.run(request).await);
            }
            Err(AuthError::DatabaseError(e)) => {
                tracing::error!(error = %e, "Session validation unavailable");
                return Err((
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Authentication unavailable",
                )
                    .into_response()
                    .into());
            }
            Err(_) => {
                cookies.remove(
                    tower_cookies::Cookie::build(SESSION_COOKIE)
                        .path("/")
                        .build(),
                );
            }
        }
    }

    if request
        .headers()
        .get("hx-request")
        .is_some_and(|h| h == "true")
    {
        return Err(([("hx-redirect", "/login")], "Authentication required")
            .into_response()
            .into());
    }
    if request.uri().path().starts_with("/api/") {
        return Err((StatusCode::UNAUTHORIZED, "Authentication required")
            .into_response()
            .into());
    }
    Err(Redirect::to("/login").into_response().into())
}

/// Middleware that requires admin role
///
/// Must be used after `require_auth` middleware.
/// Returns 403 Forbidden if user is not an admin.
pub async fn require_admin(request: Request, next: Next) -> Result<Response, HttpError> {
    // Get user from extensions (injected by require_auth)
    let user = request
        .extensions()
        .get::<User>()
        .cloned()
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Authentication required").into_response())?;

    // Check if user is admin
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, "Admin access required")
            .into_response()
            .into());
    }

    Ok(next.run(request).await)
}

/// Optional authentication middleware
///
/// Validates the session if present and injects the user, but doesn't redirect if not authenticated.
/// Always inserts Option<User> into extensions.
pub async fn optional_auth(
    Extension(db): Extension<SharedClient>,
    cookies: Cookies,
    mut request: Request,
    next: Next,
) -> Response {
    let session_id = cookies
        .get(SESSION_COOKIE)
        .map(|cookie| cookie.value().to_string());

    let user: Option<User> = if let Some(session_id) = session_id {
        match AuthService::validate_session(&db, &session_id).await {
            Ok(user) => Some(user),
            Err(AuthError::DatabaseError(e)) => {
                tracing::error!(error = %e, "Session validation unavailable");
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "Authentication unavailable",
                )
                    .into_response();
            }
            Err(_) => {
                cookies.remove(
                    tower_cookies::Cookie::build(SESSION_COOKIE)
                        .path("/")
                        .build(),
                );
                None
            }
        }
    } else {
        None
    };

    // Always insert Option<User> so handlers can check if logged in
    request.extensions_mut().insert(user);

    next.run(request).await
}

/// Helper to get authenticated user from request extensions
#[allow(dead_code)]
pub fn get_auth_user(request: &Request) -> Option<&User> {
    request.extensions().get::<User>()
}
