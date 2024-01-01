use std::collections::HashMap;

use crate::Localizer;
use fluent::FluentArgs;
use unic_langid::LanguageIdentifier;

impl tera::Function for Localizer {
    fn call(&self, args: &HashMap<String, serde_json::Value>) -> tera::Result<serde_json::Value> {
        let lang_arg = args
            .get("lang")
            .and_then(|lang| lang.as_str())
            .and_then(|str| str.parse::<LanguageIdentifier>().ok())
            .ok_or(tera::Error::msg("missing lang param"))?;

        let ftl_key = args
            .get("key")
            .and_then(|key| key.as_str())
            .ok_or(tera::Error::msg("missing ftl key"))?;

        let bundle = self
            .get_locale(&lang_arg)
            .ok_or(tera::Error::msg("locale not registered"))?;

        let msg = bundle
            .get_message(ftl_key)
            .ok_or(tera::Error::msg("FTL key not in locale"))?;
        let pattern = msg
            .value()
            .ok_or(tera::Error::msg("No value in fluent message"))?;

        let fluent_args: FluentArgs = args
            .iter()
            .filter(|(key, _)| key.as_str() != "lang" && key.as_str() != "key")
            .filter_map(|(key, val)| val.as_str().map(|val| (key, val)))
            .collect();

        let mut errs = Vec::new();
        let res = bundle.format_pattern(pattern, Some(&fluent_args), &mut errs);

        if errs.len() > 0 {
            dbg!(errs);
        }

        Ok(serde_json::Value::String(res.into()))
    }

    fn is_safe(&self) -> bool {
        true
    }
}
