//! API handlers for HTMX endpoints.

use axum::{
    Form,
    extract::{Extension, Query},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use tera::Context;
use tracing::error;

use crate::storage;
use crate::{SharedClient, SharedTera, User};

/// Example: Full page refresh endpoint
pub async fn example_refresh(Extension(tera): Extension<SharedTera>) -> Response {
    let mut ctx = Context::new();
    ctx.insert("timestamp", &chrono::Utc::now().to_rfc3339());
    ctx.insert("message", "This is a full page refresh example");

    render_template(&tera, "partials/example_refresh.html", &ctx).await
}

/// Example: Partial rendering endpoint
pub async fn example_partial(
    Query(params): Query<ExampleParams>,
    Extension(tera): Extension<SharedTera>,
) -> Response {
    let mut ctx = Context::new();
    ctx.insert("counter", &params.counter.unwrap_or(0));
    ctx.insert(
        "message",
        "This content was loaded via HTMX partial rendering",
    );

    render_template(&tera, "partials/example_partial.html", &ctx).await
}

#[derive(Debug, Deserialize)]
pub struct ExampleParams {
    counter: Option<i32>,
}

/// Get user data
pub async fn get_user_data(
    Extension(db): Extension<SharedClient>,
    Extension(user): Extension<User>,
) -> Response {
    match storage::load_user_data(&db, &user.id).await {
        Ok(Some(data)) => {
            let formatted =
                serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".to_string());
            let response = format!(
                r#"<div class="p-4 bg-blue-50 border border-blue-200 rounded-lg">
                    <p class="text-blue-700 mb-2"><strong>User Data:</strong></p>
                    <pre class="text-sm text-gray-700 bg-white p-3 rounded border overflow-auto">{}</pre>
                </div>"#,
                formatted
            );
            Html(response).into_response()
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
    Extension(db): Extension<SharedClient>,
    Extension(user): Extension<User>,
    Form(form): Form<SaveDataForm>,
) -> Response {
    let value_str = form.value.clone();
    let value: serde_json::Value = match serde_json::from_str(&value_str) {
        Ok(v) => v,
        Err(_) => serde_json::Value::String(value_str.clone()),
    };

    match storage::set_user_data_key(&db, &user.id, &form.key, &value).await {
        Ok(_) => {
            let response = format!(
                r#"<div class="p-4 bg-green-50 border border-green-200 rounded-lg">
                    <p class="text-green-700"><strong>Success!</strong> Saved: {} = {}</p>
                </div>"#,
                form.key, value_str
            );
            Html(response).into_response()
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
