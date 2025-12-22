//! Page handlers for the web application.

use axum::{
    extract::Extension,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use tera::Context;
use tracing::error;

use crate::storage;
use crate::{SharedClient, SharedTera, User};

/// Renders the home page.
pub async fn index(
    Extension(db): Extension<SharedClient>,
    Extension(tera): Extension<SharedTera>,
    Extension(user): Extension<User>,
) -> Response {
    // Load user data if available
    let user_data = storage::load_user_data(&db, &user.id)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| serde_json::json!({}));

    let mut ctx = Context::new();
    ctx.insert("user_data", &user_data);

    render_page(&tera, "pages/index.html", &ctx).await
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
pub async fn not_found(Extension(tera): Extension<SharedTera>) -> Response {
    let ctx = Context::new();
    match tera.read().await.render("404.html", &ctx) {
        Ok(html) => (StatusCode::NOT_FOUND, Html(html)).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
    }
}
