//! Admin panel handlers

use axum::{
    Form,
    extract::Extension,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use std::error::Error as StdError;
use tera::Context;

use crate::auth::{AuthService, User, UserRole};
use crate::{SharedClient, SharedTera};

/// Admin dashboard handler
pub async fn admin_dashboard(
    Extension(tera): Extension<SharedTera>,
    Extension(db): Extension<SharedClient>,
    Extension(user): Extension<User>,
) -> Result<Html<String>, Response> {
    let mut context = Context::new();
    context.insert("current_user", &user);
    
    // Add user for header navigation (same data, different key for base template)
    let user_view = serde_json::json!({
        "id": &user.id,
        "username": &user.username,
        "email": &user.email,
        "role": &user.role,
        "is_active": user.is_active,
    });
    context.insert("user", &user_view);

    // Get all users
    match AuthService::list_users(&db).await {
        Ok(users) => {
            // Calculate statistics
            let active_users_count = users.iter().filter(|u| u.is_active).count();
            let admin_users_count = users.iter().filter(|u| u.role == "admin").count();

            context.insert("users", &users);
            context.insert("active_users_count", &active_users_count);
            context.insert("admin_users_count", &admin_users_count);
        }
        Err(e) => {
            tracing::error!("Failed to list users: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to load users").into_response());
        }
    }

    let html = tera
        .read()
        .await
        .render("admin/dashboard.html", &context)
        .map_err(|e| {
            tracing::error!("Template rendering error: {}", e);
            // Log the full error chain for debugging
            let mut source: Option<&dyn StdError> = e.source();
            while let Some(err) = source {
                tracing::error!("  Caused by: {}", err);
                source = err.source();
            }
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template: {}", e),
            )
                .into_response()
        })?;

    Ok(Html(html))
}

/// Form data for updating user role
#[derive(Deserialize)]
pub struct UpdateRoleForm {
    user_id: String,
    role: String,
}

/// Update user role handler (admin only)
pub async fn update_user_role(
    Extension(db): Extension<SharedClient>,
    Form(form): Form<UpdateRoleForm>,
) -> Result<Response, Response> {
    let role = UserRole::from_str(&form.role)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid role").into_response())?;

    AuthService::update_user_role(&db, &form.user_id, role)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update user role: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update user role",
            )
                .into_response()
        })?;

    Ok((StatusCode::OK, "Role updated successfully").into_response())
}

/// Form data for toggling user status
#[derive(Deserialize)]
pub struct ToggleStatusForm {
    user_id: String,
    is_active: bool,
}

/// Toggle user active status handler (admin only)
pub async fn toggle_user_status(
    Extension(db): Extension<SharedClient>,
    Form(form): Form<ToggleStatusForm>,
) -> Result<Response, Response> {
    AuthService::toggle_user_status(&db, &form.user_id, form.is_active)
        .await
        .map_err(|e| {
            tracing::error!("Failed to toggle user status: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update user status",
            )
                .into_response()
        })?;

    Ok((StatusCode::OK, "User status updated successfully").into_response())
}
