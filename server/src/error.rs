//! A small error carrier for handlers which need early HTTP responses.
use axum::response::{IntoResponse, Response};

#[derive(Debug)]
pub struct HttpError(Box<Response>);

impl From<Response> for HttpError {
    fn from(response: Response) -> Self {
        Self(Box::new(response))
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        *self.0
    }
}
