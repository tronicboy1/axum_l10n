use std::{future::Future, pin::Pin};

use http::{HeaderMap, Request, Response, StatusCode, Uri};
use tower::{Layer, Service};
use unic_langid::LanguageIdentifier;

#[cfg(feature = "fluent")]
mod fluent;
pub use fluent::Localizer;

#[cfg(feature = "tera")]
mod tera;

/// The redirect mode for the service.
#[derive(Debug, Clone)]
pub enum RedirectMode {
    /// Does not redirect, only adds the found locale from header
    NoRedirect,
    /// Redirects to sub-path (/<lang>/*) if in list of supported Languages
    RedirectToSubPath,
}

#[derive(Debug, Clone)]
pub struct LanguageIdentifierExtractor<S> {
    inner: S,
    default_lang: LanguageIdentifier,
    supported_langs: Vec<LanguageIdentifier>,
    redirect_mode: RedirectMode,
}

impl<S> LanguageIdentifierExtractor<S> {
    pub fn new(
        inner: S,
        supported_langs: &[LanguageIdentifier],
        default_lang: &LanguageIdentifier,
    ) -> Self {
        Self {
            inner,
            default_lang: default_lang.to_owned(),
            redirect_mode: RedirectMode::NoRedirect,
            supported_langs: supported_langs.to_owned(),
        }
    }

    /// Change redirect settings of service
    pub fn redirect(mut self, redirect_mode: RedirectMode) -> Self {
        self.redirect_mode = redirect_mode;

        self
    }

    /// Unwraps the path and extracts language identifier if available.
    /// Returns None if the LanguageIdentifier is not supported
    fn lang_code_from_uri(&self, uri: &Uri) -> Option<LanguageIdentifier> {
        let mut path_parts = uri.path().split('/');
        path_parts.next();

        path_parts
            .next()
            .and_then(|code| code.parse::<LanguageIdentifier>().ok())
            .and_then(|path_ident| {
                if self.supported(&path_ident) {
                    Some(path_ident)
                } else {
                    None
                }
            })
    }

    /// Extracts language code from Accept-Language header if available and asks for at least one supported language
    ///
    /// # Details
    /// All modern browsers send the Accept-Language header to tell a server what content it should send
    ///
    /// https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Accept-Language
    fn lang_code_from_headers(&self, headers: &HeaderMap) -> Option<LanguageIdentifier> {
        let accept_lang = headers
            .get("Accept-Language")
            .and_then(|val| val.to_str().ok())?;

        accept_lang
            .parse::<LanguageIdentifier>()
            .ok()
            .and_then(|ident| {
                if self.supported(&ident) {
                    Some(ident)
                } else {
                    None
                }
            })
            .or_else(|| {
                accept_lang
                    .split(',')
                    .filter(|part| !part.is_empty())
                    // Strip the quality value
                    .map(|part| part.find(|c| c == ';').map(|i| &part[..i]).unwrap_or(part))
                    .filter_map(|ident_str| ident_str.parse::<LanguageIdentifier>().ok())
                    .find(|ident| self.supported(ident))
            })
    }

    fn supported(&self, path_ident: &LanguageIdentifier) -> bool {
        self.supported_langs
            .iter()
            .find(|ident| ident.language == path_ident.language)
            .is_some()
    }
}

impl<S, B> Service<Request<B>> for LanguageIdentifierExtractor<S>
where
    S: Service<Request<B>, Response = axum::response::Response> + Send + 'static + Clone,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Error = S::Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;
    type Response = axum::response::Response;

    /// No back pressure needed
    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        let headers = req.headers();

        let lang_ident = match &self.redirect_mode {
            RedirectMode::NoRedirect => self.lang_code_from_headers(headers),
            RedirectMode::RedirectToSubPath => self.lang_code_from_uri(req.uri()),
        };

        match &self.redirect_mode {
            &RedirectMode::NoRedirect => {
                let ident = match lang_ident {
                    Some(ident) => ident,
                    None => self.default_lang.clone(),
                };

                req.extensions_mut().insert(ident);

                Box::pin(self.inner.call(req))
            }
            &RedirectMode::RedirectToSubPath => {
                if let Some(ident) = lang_ident {
                    req.extensions_mut().insert(ident);

                    Box::pin(self.inner.call(req))
                } else {
                    let mut new_path = String::from("/");

                    if let Some(preferred_ident) = self.lang_code_from_headers(req.headers()) {
                        new_path.push_str(preferred_ident.language.to_string().as_str());
                    } else {
                        new_path.push_str(self.default_lang.language.to_string().as_str());
                    }

                    new_path.push_str(req.uri().path());

                    let response = Response::builder()
                        .status(StatusCode::FOUND)
                        .header("Location", new_path)
                        .body(axum::body::Body::empty())
                        .expect("Valid response");

                    Box::pin(async move { Ok(response) })
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct LanguageIdentifierExtractorLayer {
    default_lang: LanguageIdentifier,
    supported_langs: Vec<LanguageIdentifier>,
    redirect_mode: RedirectMode,
}

impl LanguageIdentifierExtractorLayer {
    pub fn new(
        default_lang: LanguageIdentifier,
        supported_langs: Vec<LanguageIdentifier>,
        redirect_mode: RedirectMode,
    ) -> Self {
        Self {
            default_lang,
            supported_langs,
            redirect_mode,
        }
    }
}

impl<S> Layer<S> for LanguageIdentifierExtractorLayer {
    type Service = LanguageIdentifierExtractor<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LanguageIdentifierExtractor {
            inner,
            default_lang: self.default_lang.clone(),
            supported_langs: self.supported_langs.clone(),
            redirect_mode: self.redirect_mode.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use http::HeaderValue;
    use unic_langid::langid;

    pub const ENGLISH: LanguageIdentifier = langid!("en");
    pub const JAPANESE: LanguageIdentifier = langid!("ja");

    use super::*;

    struct DummyInner;

    fn get_serv() -> LanguageIdentifierExtractor<DummyInner> {
        let supported = vec![ENGLISH, JAPANESE];
        LanguageIdentifierExtractor::new(DummyInner, &supported, &ENGLISH)
    }

    #[test]
    fn can_get_supported_lang_code_from_uri() {
        let uri = "http://localhost:3000/ja/lists".parse::<Uri>().unwrap();

        let service = get_serv();

        let ident = service.lang_code_from_uri(&uri);

        assert!(ident.is_some());
        assert_eq!(ident.unwrap(), JAPANESE)
    }

    #[test]
    fn unsupported_lang_code_from_uri() {
        let uri = "http://localhost:3000/de/lists".parse::<Uri>().unwrap();

        let service = get_serv();

        let ident = service.lang_code_from_uri(&uri);

        assert!(ident.is_none());
    }

    #[test]
    fn can_extract_lang_header_single() {
        let mut headers = HeaderMap::new();
        headers.insert("Accept-Language", HeaderValue::from_static("en"));

        let service = get_serv();

        let ident = service.lang_code_from_headers(&headers).unwrap();

        let target = "en".parse::<LanguageIdentifier>().unwrap();
        assert_eq!(ident, target)
    }

    #[test]
    fn can_extract_lang_header_compound() {
        let mut headers = HeaderMap::new();
        headers.insert("Accept-Language", HeaderValue::from_static("en-US"));

        let service = get_serv();

        let ident = service.lang_code_from_headers(&headers).unwrap();

        let target = "en-US".parse::<LanguageIdentifier>().unwrap();
        assert_eq!(ident, target)
    }

    #[test]
    fn can_extract_lang_header_compound_with_quality_val() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Accept-Language",
            HeaderValue::from_static("en-US,en;q=0.5"),
        );

        let service = get_serv();

        let ident = service.lang_code_from_headers(&headers).unwrap();

        let target = "en-US".parse::<LanguageIdentifier>().unwrap();
        assert_eq!(ident.language, target.language)
    }

    #[test]
    fn can_extract_lang_header_wildcard() {
        let mut headers = HeaderMap::new();
        headers.insert("Accept-Language", HeaderValue::from_static("*"));
    }
}
