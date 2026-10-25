//! API handlers for HTMX endpoints.

use axum::{
    Form,
    extract::{Extension, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use tera::Context;
use tracing::error;

use crate::auth::User;
use crate::storage;
use crate::{AppState, SharedTera};

/// Example: Partial rendering endpoint
pub async fn example_partial(
    Query(params): Query<ExampleParams>,
    State(state): State<AppState>,
) -> Response {
    let mut ctx = Context::new();
    let counter = params.counter.unwrap_or(1).max(0);
    ctx.insert("counter", &counter);
    ctx.insert("next_counter", &counter.saturating_add(1));
    ctx.insert(
        "message",
        "This content was loaded via HTMX partial rendering",
    );

    render_template(&state.tera, "partials/example_partial.html", &ctx).await
}

#[derive(Debug, Deserialize)]
pub struct ExampleParams {
    counter: Option<i32>,
}

/// Get user data
pub async fn get_user_data(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Response {
    match storage::load_user_data(&state.db, &user.id).await {
        Ok(Some(data)) => {
            let formatted =
                serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".to_string());
            let mut ctx = Context::new();
            ctx.insert("data", &formatted);
            render_template(&state.tera, "partials/user_data.html", &ctx).await
        }
        Ok(None) => {
            let response = r#"<div class="p-4 bg-gray-50 border border-gray-200 rounded-lg">
                <p class="text-gray-700">No data stored yet</p>
            </div>"#;
            Html(response.to_string()).into_response()
        }
        Err(e) => {
            error!(error = %e, "Failed to load user data");
            let response = r#"<div class="p-4 bg-red-50 border border-red-200 rounded-lg">
                <p class="text-red-700">Failed to load data</p>
            </div>"#;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(response.to_string()),
            )
                .into_response()
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SaveDataForm {
    key: String,
    value: String,
}

/// Save user data
pub async fn save_user_data(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Form(form): Form<SaveDataForm>,
) -> Response {
    if form.key.trim().is_empty() || form.key.len() > 128 || form.value.len() > 16 * 1024 {
        return (
            StatusCode::BAD_REQUEST,
            "Provide a key (1–128 bytes) and value (up to 16 KiB)",
        )
            .into_response();
    }
    let value_str = form.value.clone();
    let value: serde_json::Value = match serde_json::from_str(&value_str) {
        Ok(v) => v,
        Err(_) => serde_json::Value::String(value_str.clone()),
    };

    match storage::set_user_data_key(&state.db, &user.id, &form.key, &value).await {
        Ok(_) => {
            let mut ctx = Context::new();
            ctx.insert("key", &form.key);
            ctx.insert("value", &value_str);
            render_template(&state.tera, "partials/user_data.html", &ctx).await
        }
        Err(e) => {
            error!(error = %e, "Failed to save user data");
            let response = r#"<div class="p-4 bg-red-50 border border-red-200 rounded-lg">
                <p class="text-red-700">Failed to save data</p>
            </div>"#;
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(response.to_string()),
            )
                .into_response()
        }
    }
}

/// Helper to render a template with proper error handling.
async fn render_template(tera: &SharedTera, template: &str, ctx: &Context) -> Response {
    match tera.read().await.render(template, ctx) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            error!(error = %e, template = %template, "Template rendering failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "Server error").into_response()
        }
    }
}
