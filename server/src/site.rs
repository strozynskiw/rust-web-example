//! Page handlers for the web application.

use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde_json::json;
use tera::Context;
use tracing::error;

use crate::auth::User as AuthUser;
use crate::SharedTera;

/// Renders the home page (public version with optional auth user).
pub async fn index_public(
    Extension(tera): Extension<SharedTera>,
    user: Option<Extension<Option<AuthUser>>>,
) -> Response {
    let mut ctx = Context::new();
    
    // Add user to context if logged in
    if let Some(Extension(Some(auth_user))) = user {
        let user_view = json!({
            "id": &auth_user.id,
            "username": &auth_user.username,
            "email": &auth_user.email,
            "role": &auth_user.role,
            "is_active": auth_user.is_active,
        });
        ctx.insert("user", &user_view);
    }

    render_page(&state.tera, "pages/index.html", &ctx).await
}

/// Helper to render a template with proper error handling.
async fn render_page(tera: &SharedTera, template: &str, ctx: &Context) -> Response {
    match tera.read().await.render(template, ctx) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            error!(error = %e, template = %template, "Template rendering failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}

/// 404 Not Found handler.
pub async fn not_found(State(state): State<AppState>) -> Response {
    let ctx = Context::new();
    match state.tera.read().await.render("404.html", &ctx) {
        Ok(html) => (StatusCode::NOT_FOUND, Html(html)).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
    }
}

/// User profile page (requires authentication).
pub async fn user_profile(
    Extension(tera): Extension<SharedTera>,
    Extension(user): Extension<AuthUser>,
) -> Response {
    // Create user view for template
    let user_view = json!({
        "id": &user.id,
        "username": &user.username,
        "email": &user.email,
        "role": &user.role,
        "is_active": user.is_active,
        "created_at": user.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        "last_login_at": user.last_login_at.map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string()),
    });

    let mut ctx = Context::new();
    ctx.insert("user", &user_view);

    render_page(&tera, "pages/profile.html", &ctx).await
}
