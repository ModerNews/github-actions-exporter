//! HMAC-SHA256 verification of `X-Hub-Signature-256`.
//!
//! The tag is compared in constant time by `verify_slice`, against the raw
//! body bytes — never a deserialized payload.

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use crate::errors::signature_error::SignatureError;
use crate::http::headers::{FromHeader, ParseHeader};

type HmacSha256 = Hmac<Sha256>;

pub struct Signature([u8; 32]);

#[derive(Clone)]
pub struct Verifier(HmacSha256);

impl Verifier {
    pub fn new(secret: &str) -> anyhow::Result<Self> {
        Ok(Self(HmacSha256::new_from_slice(secret.as_bytes())?))
    }

    pub fn verify(&self, sig: &Signature, body: &[u8]) -> Result<(), SignatureError> {
        let mut mac = self.0.clone();
        mac.update(body);
        mac.verify_slice(&sig.0)
            .map_err(|_| SignatureError::Invalid)
    }
}

impl FromHeader for Signature {
    const HEADER: &'static str = "x-hub-signature-256";
}

impl ParseHeader for Signature {
    fn parse_header(value: &str) -> Option<Self> {
        let stripped = value.strip_prefix("sha256=")?;
        let mut tag = [0u8; 32];
        hex::decode_to_slice(stripped, &mut tag).ok()?;
        Some(Self(tag))
    }
}
