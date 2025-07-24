use std::{collections::HashMap, error::Error, fmt::Debug, path::Path};

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
        let full_locale_match = self.locales.get(locale);

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
    pub fn format_message(
        &self,
        locale: &LanguageIdentifier,
        key: &(impl MessageKey + ?Sized),
        args: Option<&FluentArgs>,
    ) -> Option<String> {
        self.format_message_result(locale, key, args).ok()
    }

    /// Format a FTL message into target locale if available.<br>
    /// See Fluent RS [FluentBundle::format_pattern documentation](https://docs.rs/fluent/latest/fluent/bundle/struct.FluentBundle.html#method.format_pattern)
    /// for details
    ///
    /// Fluent template errors are printed to stdout.
    pub fn format_message_result(
        &self,
        locale: &LanguageIdentifier,
        key: &(impl MessageKey + ?Sized),
        args: Option<&FluentArgs>,
    ) -> Result<String, Box<dyn Error + Send + Sync + 'static>> {
        let bundle = self
            .get_locale(locale)
            .ok_or_else(|| format!("could not find locale {locale}"))?;

        let message = bundle
            .get_message(key.key())
            .ok_or_else(|| format!("could not find message with key={}", key.key()))?;

        let mut errors = Vec::new();

        let message = if let Some(attribute) = key.attribute() {
            let attribute = message.get_attribute(attribute).ok_or_else(|| {
                format!(
                    "could not find attribute={attribute} for message with key={}",
                    key.key()
                )
            })?;

            bundle
                .format_pattern(attribute.value(), args, &mut errors)
                .to_string()
        } else {
            bundle
                .format_pattern(
                    message.value().ok_or_else(|| {
                        format!(
                            "message with key={} does not have a standalone message",
                            key.key()
                        )
                    })?,
                    args,
                    &mut errors,
                )
                .to_string()
        };

        for err in errors {
            #[cfg(not(feature = "tracing"))]
            println!("{}", err);
            #[cfg(feature = "tracing")]
            tracing::warn!("Fluent error: {}", err);
        }

        Ok(message)
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

pub trait MessageKey {
    fn key(&self) -> &str;

    fn attribute(&self) -> Option<&str> {
        None
    }
}

impl<S: AsRef<str> + ?Sized> MessageKey for S {
    fn key(&self) -> &str {
        self.as_ref()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MessageAttribute<'key, 'attribute> {
    pub key: &'key str,
    pub attribute: &'attribute str,
}

impl<'key, 'attribute> MessageKey for MessageAttribute<'key, 'attribute> {
    fn key(&self) -> &str {
        self.key
    }

    fn attribute(&self) -> Option<&str> {
        Some(self.attribute)
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
    fn compiles_with_borrowed_string() {
        let mut loc = Localizer::new();
        loc.add_bundle(ENGLISH, &[MAIN, SUB]).unwrap();

        loc.format_message(&ENGLISH, &"test-key-a".to_owned(), None);
    }

    #[test]
    fn use_attributes() {
        let mut loc = Localizer::new();
        loc.add_bundle(ENGLISH, &[MAIN, SUB]).unwrap();

        let message = loc
            .format_message(
                &ENGLISH,
                &MessageAttribute {
                    key: "attribute-test",
                    attribute: "attribute_a",
                },
                None,
            )
            .expect("formatting succeeds");

        assert_eq!("Hello", message)
    }

    #[test]
    fn not_existing_attribute() {
        let mut loc = Localizer::new();
        loc.add_bundle(ENGLISH, &[MAIN, SUB]).unwrap();

        assert!(loc
            .format_message(
                &ENGLISH,
                &MessageAttribute {
                    key: "attribute-test",
                    attribute: "does_not_exist",
                },
                None,
            )
            .is_none());
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
