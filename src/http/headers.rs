//! Typed header extraction.
//!
//! [`ParseHeader`] is the parse itself; [`FromHeader`] additionally pins the
//! header a type is read from, so the call site names neither.

use axum::http::HeaderMap;

use crate::errors::header_error::HeaderError;

pub trait ParseHeader: Sized {
    fn parse_header(value: &str) -> Option<Self>;
}

pub trait FromHeader: ParseHeader {
    const HEADER: &'static str;
}

pub trait RequireHeader {
    fn require<T: FromHeader>(&self) -> Result<T, HeaderError>;
    fn require_named<T: ParseHeader>(&self, name: &'static str) -> Result<T, HeaderError>;
}

impl RequireHeader for HeaderMap {
    fn require<T: FromHeader>(&self) -> Result<T, HeaderError> {
        self.require_named(T::HEADER)
    }

    fn require_named<T: ParseHeader>(&self, name: &'static str) -> Result<T, HeaderError> {
        self.get(name)
            .ok_or(HeaderError::Missing(name))?
            .to_str()
            .ok()
            .and_then(T::parse_header)
            .ok_or(HeaderError::Invalid(name))
    }
}

impl ParseHeader for String {
    fn parse_header(value: &str) -> Option<Self> {
        Some(value.to_owned())
    }
}
