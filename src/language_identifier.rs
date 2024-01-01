use axum::response::{IntoResponse, Redirect, Response};
use http::{HeaderMap, Request, Uri};
use serde::ser::Serialize;
use std::future::Future;
use std::pin::Pin;
use tower::{Layer, Service};
use unic_langid::LanguageIdentifier;

use super::{supported, ENGLISH};

/// Newtype of unic_langid::LanguageIdentifier to allow serialization in use with Tera
#[derive(Debug, Clone)]
pub struct TeraLanguageIdentifier(LanguageIdentifier);

impl Serialize for TeraLanguageIdentifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.language.to_string().as_str())
    }
}

impl std::ops::Deref for TeraLanguageIdentifier {
    type Target = LanguageIdentifier;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

