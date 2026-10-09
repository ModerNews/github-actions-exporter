use axum::http::StatusCode;

pub enum HeaderError {
    Missing(&'static str),
    Invalid(&'static str),
}

impl HeaderError {
    pub fn to_http(&self) -> (StatusCode, String) {
        match self {
            Self::Missing(h) => (StatusCode::BAD_REQUEST, format!("missing header: {h}")),
            Self::Invalid(h) => (StatusCode::BAD_REQUEST, format!("invalid header: {h}")),
        }
    }
}
