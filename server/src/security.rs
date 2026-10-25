//! Same-origin form protection and deployment-aware cookies.
use axum::{
    extract::{Extension, Request},
    http::{HeaderValue, Method, StatusCode, Uri},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Debug)]
pub struct AuthWindow {
    started: Instant,
    attempts: u32,
}

#[derive(Clone, Debug)]
pub struct SecurityConfig {
    pub origin: HeaderValue,
    pub secure_cookies: bool,
    pub auth_window: Arc<Mutex<AuthWindow>>,
}

impl SecurityConfig {
    pub fn new(origin: &str) -> anyhow::Result<Self> {
        let uri: Uri = origin.parse()?;
        anyhow::ensure!(
            matches!(uri.scheme_str(), Some("http" | "https"))
                && uri.authority().is_some()
                && uri.path() == "/"
                && uri.query().is_none()
                && !origin.ends_with('/')
                && !origin.contains('@'),
            "APP_ORIGIN must be an http(s) origin without credentials, path or trailing slash"
        );
        Ok(Self {
            origin: origin.parse()?,
            secure_cookies: uri.scheme_str() == Some("https"),
            auth_window: Arc::new(Mutex::new(AuthWindow {
                started: Instant::now(),
                attempts: 0,
            })),
        })
    }
}

/// Unsafe requests, including login, must have an exact configured Origin.
/// Host and forwarded headers are never used to determine the trusted origin.
pub async fn same_origin(
    Extension(config): Extension<SecurityConfig>,
    request: Request,
    next: Next,
) -> Response {
    if !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    ) && request.headers().get("origin") != Some(&config.origin)
    {
        return (
            StatusCode::FORBIDDEN,
            "A matching Origin header is required",
        )
            .into_response();
    }
    let is_static = request.uri().path().starts_with("/static/");
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("content-security-policy", HeaderValue::from_static(
        "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"));
    if config.secure_cookies {
        headers.insert(
            "strict-transport-security",
            HeaderValue::from_static("max-age=31536000"),
        );
    }
    if !is_static {
        headers.insert("cache-control", HeaderValue::from_static("no-store"));
    }
    response
}

/// Small single-process admission budget; configure per-client limits at the proxy too.
pub async fn auth_capacity(
    Extension(config): Extension<SecurityConfig>,
    request: Request,
    next: Next,
) -> Response {
    let allowed = {
        let Ok(mut window) = config.auth_window.lock() else {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "Authentication unavailable",
            )
                .into_response();
        };
        if window.started.elapsed() >= Duration::from_secs(60) {
            window.started = Instant::now();
            window.attempts = 0;
        }
        window.attempts += 1;
        window.attempts <= 30
    };
    if !allowed {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [("retry-after", "60")],
            "Too many authentication attempts; retry shortly",
        )
            .into_response();
    }
    next.run(request).await
}
