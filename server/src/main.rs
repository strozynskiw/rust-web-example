//! web-template
//!
//! A web application built with:
//! - Axum for HTTP handling
//! - Tera for templating
//! - SQLite/PostgreSQL for storage (SQLite by default)
//! - HTMX for dynamic interactions
//! - Tailwind CSS for styling

use axum::http::HeaderValue;
use axum::{
    Router,
    extract::{Extension, Request},
    middleware::Next,
    response::Response,
    routing::{get, post},
};
use std::sync::Arc;
use tera::Tera;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tracing::info;
use uuid::Uuid;

use web_template_common::db;

mod admin;
mod api;
mod auth;
mod auth_handlers;
mod middlewares;
mod site;
mod storage;

/// Cookie name used to identify users across sessions.
const USER_ID_COOKIE: &str = "user_id";

/// Default server port.
const DEFAULT_SERVER_PORT: u16 = 3000;

/// Represents an authenticated user identified by a UUID.
#[derive(Clone, Debug)]
pub struct User {
    pub id: Uuid,
}

/// Shared database connection pool (supports both SQLite and PostgreSQL).
pub type SharedClient = web_template_common::db::DatabasePool;

/// Thread-safe shared Tera template engine with hot-reloading support.
pub type SharedTera = Arc<RwLock<Tera>>;

/// Application state shared across all handlers.
#[derive(Clone)]
pub struct AppState {
    pub tera: SharedTera,
    pub db: SharedClient,
}

/// Initialize the Tera template engine.
fn setup_tera() -> anyhow::Result<SharedTera> {
    let mut tera = Tera::default();
    tera.load_from_glob("templates/**/*")?;
    let template_count = tera.get_template_names().count();
    info!("   Found {} template(s)", template_count);
    Ok(Arc::new(RwLock::new(tera)))
}

/// Initialize the database connection pool and run migrations.
async fn setup_database() -> anyhow::Result<SharedClient> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| {
            tracing::warn!("DATABASE_URL not set, using default: sqlite:./data.db");
            "sqlite:./data.db".to_string()
        });
    
    // Determine database type
    let db_type = if database_url.starts_with("sqlite") {
        "SQLite"
    } else if database_url.starts_with("postgres") {
        "PostgreSQL"
    } else {
        "Unknown"
    };
    
    info!("   Database type: {}", db_type);
    let pool = db::create_pool(&database_url).await?;
    info!("   Connection pool created");

    // Run migrations based on database type
    info!("   Running migrations...");
    match &pool {
        web_template_common::db::DatabasePool::Sqlite(sqlite_pool) => {
            sqlx::migrate!("./migrations").run(sqlite_pool).await?;
            info!("   SQLite migrations applied");
        }
        web_template_common::db::DatabasePool::Postgres(postgres_pool) => {
            sqlx::migrate!("./migrations").run(postgres_pool).await?;
            info!("   PostgreSQL migrations applied");
        }
    }

    Ok(pool)
}

/// Initialize the tracing subscriber for structured logging.
fn setup_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "web_template=info,tower_http=info".into()),
        )
        .with_target(false)
        .with_level(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .init();
}

/// Build the application router with all routes and middleware.
fn build_router(state: AppState) -> Router {
    // Public routes (no authentication required, but check if user is logged in)
    let public_routes = Router::new()
        .route("/", get(site::index_public))
        .route("/login", get(auth_handlers::login_page))
        .route("/register", get(auth_handlers::register_page))
        // Add optional auth middleware to inject user if logged in
        .layer(axum::middleware::from_fn(middlewares::optional_auth));
    
    // Auth action routes (login/logout/register actions)
    let auth_action_routes = Router::new()
        .route("/login", post(auth_handlers::login))
        .route("/register", post(auth_handlers::register))
        .route("/logout", get(auth_handlers::logout));
    
    // HTMX example routes
    let example_routes = Router::new()
        .route("/api/example/refresh", get(api::example_refresh))
        .route("/api/example/partial", get(api::example_partial));

    // Protected user routes (authentication required)
    let user_routes = Router::new()
        .route("/profile", get(site::user_profile))
        .route("/dashboard", get(site::user_profile))
        .route("/api/user-data", get(api::get_user_data))
        .route("/api/user-data", post(api::save_user_data))
        .layer(axum::middleware::from_fn(middlewares::require_auth));

    // Admin routes (authentication + admin role required)
    let admin_routes = Router::new()
        .route("/admin", get(admin::admin_dashboard))
        .route("/admin/users/role", post(admin::update_user_role))
        .route("/admin/users/status", post(admin::toggle_user_status))
        .layer(axum::middleware::from_fn(middlewares::require_admin))
        .layer(axum::middleware::from_fn(middlewares::require_auth));

    Router::new()
        .merge(public_routes)
        .merge(auth_action_routes)
        .merge(example_routes)
        .merge(user_routes)
        .merge(admin_routes)
        // Static files with cache headers
        .nest_service(
            "/static",
            ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=31536000, immutable"),
                ))
                .service(ServeDir::new("static")),
        )
        // Fallback for 404
        .fallback(site::not_found)
        // Middleware
        .layer(middleware::from_fn(tera_reload_middleware))
        .layer(Extension(state.tera.clone()))
        .with_state(state)
        .layer(middleware::from_fn(user_identity_middleware))
        .layer(CookieManagerLayer::new())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables first
    dotenv::dotenv().ok();

    // Initialize logging early
    setup_tracing();

    // Startup banner
    info!("");
    info!("🚀 Web Template Server Starting...");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let tera = setup_tera()?;
    let db = setup_database().await?;
    let state = AppState { tera, db };

    let app = build_router(state);

    // Start the server
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_SERVER_PORT);
    let addr = format!("0.0.0.0:{}", port);
    
    info!("🌐 Starting HTTP server...");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("✨ Server ready and listening on {}", addr);
    info!("📍 Access the application at:");
    info!("   • Home: http://localhost:{}", port);
    info!("   • Login: http://localhost:{}/login", port);
    info!("   • Register: http://localhost:{}/register", port);
    info!("   • Admin: http://localhost:{}/admin", port);
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("");

    axum::serve(listener, app).await?;

    Ok(())
}

/// Middleware that hot-reloads Tera templates in debug mode.
#[allow(unused_variables)]
async fn tera_reload_middleware(
    Extension(tera): Extension<SharedTera>,
    request: Request,
    next: Next,
) -> Response {
    #[cfg(debug_assertions)]
    {
        let mut t = tera.write().await;
        if let Err(e) = t.full_reload() {
            tracing::warn!(error = %e, "Failed to reload templates");
        }
    }
    next.run(request).await
}

/// Ensures each user has a persistent UUID identifier.
async fn user_identity_middleware(mut request: Request, next: Next) -> Response {
    use tower_cookies::Cookies;

    let cookies = request.extensions().get::<Cookies>().cloned();
    if let Some(cookies) = cookies {
        let id = get_or_create_user_id(&cookies);
        request.extensions_mut().insert(User { id });
    }
    next.run(request).await
}

/// Retrieves an existing user ID from cookies or creates a new one.
fn get_or_create_user_id(cookies: &tower_cookies::Cookies) -> Uuid {
    use std::str::FromStr;
    use tower_cookies::Cookie;

    if let Some(cookie) = cookies.get(USER_ID_COOKIE)
        && let Ok(id) = Uuid::from_str(cookie.value())
    {
        return id;
    }

    // Create new user ID and cookie
    let id = Uuid::new_v4();
    let cookie = Cookie::build((USER_ID_COOKIE.to_owned(), id.to_string()))
        .permanent()
        .path("/")
        .build();
    cookies.add(cookie);
    id
}
