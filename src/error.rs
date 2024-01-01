use std::convert::Infallible;

pub struct LanguageIdentifierExtractorError {}

impl std::fmt::Display for LanguageIdentifierExtractorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to extract language identifier from request.")
    }
}

impl std::fmt::Debug for LanguageIdentifierExtractorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.to_string())
    }
}

impl std::error::Error for LanguageIdentifierExtractorError {}

impl From<Infallible> for LanguageIdentifierExtractorError {
    fn from(value: Infallible) -> Self {
        Self {}
    }
}
