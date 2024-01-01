use std::{collections::HashMap, fmt::Debug, path::Path};

use fluent::{bundle::FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

pub type Bundle = FluentBundle<FluentResource, intl_memoizer::concurrent::IntlLangMemoizer>;

pub type Locales = HashMap<LanguageIdentifier, Bundle>;

pub struct Localizer {
    locales: Locales,
}

#[derive(Debug)]
pub struct LocalizerError {
    cause: String,
}

impl LocalizerError {
    fn new(cause: String) -> Self {
        Self { cause }
    }
}

impl std::fmt::Display for LocalizerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Localizer error: {}", self.cause)
    }
}

impl std::error::Error for LocalizerError {}

impl Localizer {
    pub fn new() -> Self {
        let locales = HashMap::new();

        Self { locales }
    }

    /// Adds a bundle to the localizer including all the FTL files given by their file paths
    pub fn add_bundle<P>(
        &mut self,
        locale: LanguageIdentifier,
        ftl_paths: &[P],
    ) -> Result<(), LocalizerError>
    where
        P: Debug + AsRef<Path>,
    {
        let mut bundle = FluentBundle::new_concurrent(vec![locale.clone()]);

        for path in ftl_paths {
            let ftl = std::fs::read_to_string(path).map_err(|_err| {
                LocalizerError::new(format!("failed to read from path: {:?}", path))
            })?;
            let ftl = FluentResource::try_new(ftl).map_err(|err| {
                LocalizerError::new(format!(
                    "failed to parse FTL: {:?}, with reason: {:?}",
                    path, err.1
                ))
            })?;

            bundle.add_resource(ftl).map_err(|err| {
                LocalizerError::new(format!("Unable to add resource: {:?}", path))
            })?;
        }

        self.locales.insert(locale, bundle);

        Ok(())
    }

    /// Searches for a full locale match and returns it.
    /// If no full locale match, returns a language match if available
    pub fn get_locale(&self, locale: &LanguageIdentifier) -> Option<&Bundle> {
        let full_locale_match = self.locales.get(&*locale);

        // Try to match only on the language if full match not found
        if full_locale_match.is_some() {
            full_locale_match
        } else {
            if let Some(key) = self.locales.keys().find(|k| k.language == locale.language) {
                self.locales.get(key)
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unic_langid::langid;

    const ENGLISH: LanguageIdentifier = langid!("en");
    const JAPANESE: LanguageIdentifier = langid!("ja");
    const MAIN: &str = "test_data/main.ftl";
    const SUB: &str = "test_data/sub.ftl";

    #[test]
    fn can_add_bundles() {
        let mut loc = Localizer::new();
        loc.add_bundle(ENGLISH, &[MAIN, SUB]).unwrap();
        loc.add_bundle(JAPANESE, &[MAIN, SUB]).unwrap();
    }

    #[test]
    fn can_get_bundles() {
        let mut loc = Localizer::new();
        loc.add_bundle(ENGLISH, &[MAIN, SUB]).unwrap();
        loc.add_bundle(JAPANESE, &[MAIN, SUB]).unwrap();

        let bundle = loc.get_locale(&ENGLISH);

        assert!(bundle.is_some());
    }

    #[test]
    fn can_get_bundle_on_lang() {
        let mut loc = Localizer::new();
        loc.add_bundle(ENGLISH, &[MAIN, SUB]).unwrap();
        loc.add_bundle(JAPANESE, &[MAIN, SUB]).unwrap();

        let bundle = loc.get_locale(&langid!("en-US"));

        assert!(bundle.is_some());
    }
}
