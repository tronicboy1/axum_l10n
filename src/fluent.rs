use std::{collections::HashMap, fmt::Debug, path::Path};

use fluent::{bundle::FluentBundle, types::FluentNumberOptions, FluentArgs, FluentResource};
use unic_langid::LanguageIdentifier;

pub type Bundle = FluentBundle<FluentResource, intl_memoizer::concurrent::IntlLangMemoizer>;

pub type Locales = HashMap<LanguageIdentifier, Bundle>;

pub struct Localizer {
    locales: Locales,
    number_options: FluentNumberOptions,
}

impl std::fmt::Debug for Localizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Localizer - {}",
            self.locales
                .keys()
                .map(|k| k.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
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

        Self {
            locales,
            number_options: FluentNumberOptions::default(),
        }
    }

    /// Set fluent number conversion options
    pub fn set_fluent_number_options(mut self, number_options: FluentNumberOptions) -> Self {
        self.number_options = number_options;

        self
    }

    pub fn number_options(&self) -> &FluentNumberOptions {
        &self.number_options
    }

    /// Adds a bundle to the localizer including all the FTL files given by their file paths
    ///
    /// If subsequent files contain the same keys as previous ones, those messages will be
    /// overwritten by the later value.
    /// You may use this to provide "fallback" translations, followed by the actual main
    /// translation.
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

            bundle.add_resource_overriding(ftl);
        }

        self.locales.insert(locale, bundle);

        Ok(())
    }

    /// Searches for a full locale match and returns it.
    /// If no full locale match, returns a language match if available
    pub fn get_locale(&self, locale: &LanguageIdentifier) -> Option<&Bundle> {
        let full_locale_match = self.locales.get(&*locale);

        // Try to match only on the language if full match not found
        match full_locale_match {
            Some(l) => Some(l),
            None => self
                .locales
                .keys()
                .find(|k| k.language == locale.language)
                .and_then(|key| self.locales.get(key)),
        }
    }

    /// Format a FTL message into target locale if available.<br>
    /// See Fluent RS [FluentBundle::format_pattern documentation](https://docs.rs/fluent/latest/fluent/bundle/struct.FluentBundle.html#method.format_pattern)
    /// for details
    ///
    /// Fluent template errors are printed to stdout.
    pub fn format_message<'a>(
        &self,
        locale: &LanguageIdentifier,
        key: &str,
        args: Option<&'a FluentArgs>,
    ) -> Option<String> {
        let bundle = self.get_locale(locale)?;

        let message = bundle.get_message(key)?;

        let pattern = message.value()?;

        let mut errors = Vec::new();

        let message = bundle
            .format_pattern(pattern, args, &mut errors)
            .to_string();

        if errors.len() > 0 {
            for err in errors {
                println!("{}", err.to_string());
            }
        }

        Some(message)
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<LanguageIdentifier, Bundle> {
        self.locales.iter()
    }

    /// Use to iter all registered bundles and add functions or other
    /// customizations.
    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<LanguageIdentifier, Bundle> {
        self.locales.iter_mut()
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

    #[test]
    fn can_format_pattern() {
        let mut loc = Localizer::new();
        loc.add_bundle(ENGLISH, &[MAIN, SUB]).unwrap();

        let message = loc.format_message(&ENGLISH, "test-key-a", None);

        assert_eq!(Some(String::from("Hello World")), message);

        let mut args = fluent::FluentArgs::new();
        args.set("name", "Deadpool");

        let message = loc.format_message(&ENGLISH, "test-name", Some(&args));

        assert_eq!(Some(String::from("Peg \u{2068}Deadpool\u{2069}")), message);
    }
}
