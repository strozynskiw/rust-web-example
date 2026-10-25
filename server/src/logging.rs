//! Observable by default, even when RUST_LOG names an outdated application target.
use axum::{extract::Request, middleware::Next, response::Response};
use std::{io::IsTerminal, time::Instant};
use tracing_subscriber::EnvFilter;

fn log_filter(value: Option<&str>) -> Result<EnvFilter, tracing_subscriber::filter::ParseError> {
    // Prepend an actual global directive: the builder's default directive is
    // only used for an empty filter, not for a nonempty target-only filter.
    // Explicit global/target directives in RUST_LOG still override this baseline.
    EnvFilter::try_new(format!("info,{}", value.unwrap_or_default()))
}

pub fn setup() {
    let value = std::env::var("RUST_LOG").ok();
    let filter = log_filter(value.as_deref()).unwrap_or_else(|_| {
        eprintln!("Invalid RUST_LOG filter; falling back to INFO logging");
        EnvFilter::new("info")
    });
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .with_target(false)
        .init();
}

/// Do not record query strings, headers, cookies or bodies in access logs.
pub async fn access(request: Request, next: Next) -> Response {
    let started = Instant::now();
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let response = next.run(request).await;
    tracing::info!(
        target: "web_template::http",
        %method, %path, status = response.status().as_u16(),
        elapsed_ms = started.elapsed().as_millis(),
        "HTTP request completed"
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info_enabled(value: Option<&str>) -> bool {
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(log_filter(value).unwrap())
            .with_writer(std::io::sink)
            .finish();
        tracing::subscriber::with_default(
            subscriber,
            || tracing::enabled!(target: "web_template", tracing::Level::INFO),
        )
    }

    #[test]
    fn default_and_legacy_target_keep_startup_visible() {
        assert!(info_enabled(None));
        assert!(info_enabled(Some("")));
        assert!(info_enabled(Some(
            "rust_web_template=debug,tower_http=debug"
        )));
        assert!(info_enabled(Some("web_template=info")));
    }

    #[test]
    fn explicit_quiet_filters_are_respected() {
        assert!(!info_enabled(Some("warn")));
        assert!(!info_enabled(Some("off")));
        assert!(!info_enabled(Some("web_template=error")));
        assert!(log_filter(Some("web_template=not-a-level")).is_err());
    }
}
