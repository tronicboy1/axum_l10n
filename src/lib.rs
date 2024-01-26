use std::{future::Future, pin::Pin};

use http::{HeaderMap, Response, StatusCode, Uri};
use tower::{Layer, Service};
use unic_langid::LanguageIdentifier;

#[cfg(feature = "fluent")]
mod fluent;
#[cfg(feature = "fluent")]
pub use fluent::Localizer;

#[cfg(feature = "tera")]
mod tera;

/// The redirect mode for the service.
#[derive(Debug, Clone)]
pub enum RedirectMode {
    /// Does not redirect, only adds the found locale from header
    NoRedirect,
    /// Redirects to sub-path (/<lang>-<region>/*) if in list of supported Languages
    /// Ex. localhost:3000/lists -> localhost:3000/en-US/lists
    RedirectToFullLocaleSubPath,
    /// Redirects to sub-path (/<lang>/*) if in list of supported Languages
    /// Ex. localhost:3000/lists -> localhost:3000/en/lists
    RedirectToLanguageSubPath,
}

#[derive(Debug, Clone)]
pub struct LanguageIdentifierExtractor<S> {
    inner: S,
    default_lang: LanguageIdentifier,
    supported_langs: Vec<LanguageIdentifier>,
    redirect_mode: RedirectMode,
    excluded_paths: Vec<String>,
}

macro_rules! builder_funcs {
    () => {
        /// Change redirect settings of service
        pub fn redirect(mut self, redirect_mode: RedirectMode) -> Self {
            self.redirect_mode = redirect_mode;

            self
        }

        /// Exclude paths from redirect when in Redirect mode
        /// Must use paths that start with `/`.
        ///
        /// # Example
        /// ```ignore
        /// let layer = axum_l10n::LanguageIdentifierExtractorLayer::new(
        ///     ENGLISH,
        ///     vec![ENGLISH, JAPANESE],
        ///     axum_l10n::RedirectMode::RedirectToLanguageSubPath,
        /// ).excluded_paths(&["/.well-known", ])
        /// ```
        pub fn excluded_paths(mut self, paths_to_exclude: &[&str]) -> Self {
            self.excluded_paths = paths_to_exclude
                .into_iter()
                .map(|v| v.to_string())
                .collect();

            self
        }
    };
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
            excluded_paths: Vec::new(),
        }
    }

    builder_funcs!();

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

    // Rewrites uri without the language code
    fn rewrite_uri(
        &self,
        uri: &mut http::Uri,
        ident: &LanguageIdentifier,
    ) -> Result<(), http::uri::InvalidUri> {
        let lang_code = match &self.redirect_mode {
            RedirectMode::RedirectToFullLocaleSubPath => ident.to_string(),
            RedirectMode::RedirectToLanguageSubPath => ident.language.to_string(),
            RedirectMode::NoRedirect => unreachable!(),
        };

        let new_uri = uri
            .to_string()
            .replacen(&format!("/{}/", lang_code), "/", 1);
        *uri = http::Uri::try_from(new_uri)?;

        Ok(())
    }

    fn build_redirect_path<B>(&self, req: &http::Request<B>) -> String {
        let mut new_path = String::from("/");

        let ident = if let Some(preferred_ident) = self.lang_code_from_headers(req.headers()) {
            preferred_ident
        } else {
            self.default_lang.clone()
        };
        let ident_string = match self.redirect_mode {
            RedirectMode::RedirectToFullLocaleSubPath => ident.to_string(),
            RedirectMode::RedirectToLanguageSubPath => ident.language.to_string(),
            _ => unreachable!(),
        };

        new_path.push_str(&ident_string);
        new_path.push_str(req.uri().path());

        if let Some(q) = req.uri().query() {
            new_path.push_str("?");
            new_path.push_str(q);
        }

        new_path
    }
}

impl<S, B> Service<http::Request<B>> for LanguageIdentifierExtractor<S>
where
    S: Service<http::Request<B>, Response = axum::response::Response> + Send + 'static + Clone,
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

    fn call(&mut self, mut req: http::Request<B>) -> Self::Future {
        let headers = req.headers();

        let lang_ident = match &self.redirect_mode {
            RedirectMode::NoRedirect => self.lang_code_from_headers(headers),
            RedirectMode::RedirectToLanguageSubPath | RedirectMode::RedirectToFullLocaleSubPath => {
                self.lang_code_from_uri(req.uri())
            }
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
            RedirectMode::RedirectToFullLocaleSubPath | RedirectMode::RedirectToLanguageSubPath => {
                if let Some(ident) = lang_ident {
                    // Remove lang code from path for matching in axum
                    let uri = req.uri_mut();
                    self.rewrite_uri(uri, &ident).expect("invalid url");

                    req.extensions_mut().insert(ident);

                    Box::pin(self.inner.call(req))
                } else {
                    // Do not redirect if in excluded paths
                    let path = req.uri().path();
                    if self
                        .excluded_paths
                        .iter()
                        .any(|excluded| path.starts_with(excluded))
                    {
                        return Box::pin(self.inner.call(req));
                    }

                    let new_path = self.build_redirect_path(&req);

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
    excluded_paths: Vec<String>,
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
            excluded_paths: Vec::new(),
        }
    }

    builder_funcs!();
}

impl<S> Layer<S> for LanguageIdentifierExtractorLayer {
    type Service = LanguageIdentifierExtractor<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LanguageIdentifierExtractor {
            inner,
            default_lang: self.default_lang.clone(),
            supported_langs: self.supported_langs.clone(),
            redirect_mode: self.redirect_mode.clone(),
            excluded_paths: self.excluded_paths.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

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
    fn can_rewrite_uri_full() {
        let mut uri = "http://localhost:3000/en-US/lists".parse::<Uri>().unwrap();

        let mut service = get_serv();
        service.redirect_mode = RedirectMode::RedirectToFullLocaleSubPath;

        let ident = LanguageIdentifier::from_str("en-US").unwrap();

        service.rewrite_uri(&mut uri, &ident).unwrap();

        assert_eq!("http://localhost:3000/lists", uri.to_string().as_str());
    }

    #[test]
    fn can_rewrite_uri_lang_only() {
        let mut uri = "http://localhost:3000/en/lists".parse::<Uri>().unwrap();

        let mut service = get_serv();
        service.redirect_mode = RedirectMode::RedirectToLanguageSubPath;

        let ident = LanguageIdentifier::from_str("en-US").unwrap();

        service.rewrite_uri(&mut uri, &ident).unwrap();

        assert_eq!("http://localhost:3000/lists", uri.to_string().as_str());
    }

    #[test]
    fn can_rewrite_uri_same_starting_text() {
        let mut uri = "http://localhost:3000/en/enrollment/details"
            .parse::<Uri>()
            .unwrap();

        let mut service = get_serv();
        service.redirect_mode = RedirectMode::RedirectToLanguageSubPath;

        let ident = LanguageIdentifier::from_str("en-US").unwrap();

        service.rewrite_uri(&mut uri, &ident).unwrap();

        assert_eq!(
            "http://localhost:3000/enrollment/details",
            uri.to_string().as_str()
        );
    }

    #[test]
    fn can_rewrite_uri_with_query_params() {
        let mut uri = "http://localhost:3000/en/?page=1".parse::<Uri>().unwrap();

        let mut service = get_serv();
        service.redirect_mode = RedirectMode::RedirectToLanguageSubPath;

        let ident = LanguageIdentifier::from_str("en-US").unwrap();

        service.rewrite_uri(&mut uri, &ident).unwrap();

        assert_eq!("http://localhost:3000/?page=1", uri.to_string().as_str());
    }

    #[test]
    fn can_redirect_with_query_params() {
        let uri = "http://localhost:3000/?page=1".parse::<Uri>().unwrap();
        let req = http::Request::builder()
            .uri(uri)
            .header("Accept-Language", "en-US,en;q=0.5")
            .body(())
            .unwrap();

        let mut service = get_serv();
        service.redirect_mode = RedirectMode::RedirectToLanguageSubPath;

        let ident = LanguageIdentifier::from_str("en-US").unwrap();

        let new_path = service.build_redirect_path(&req);

        assert_eq!("/en/?page=1", new_path.as_str());
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
