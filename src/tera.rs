use std::{borrow::Cow, collections::HashMap};

use crate::Localizer;
use fluent::{
    types::{FluentNumber, FluentNumberOptions},
    FluentArgs, FluentValue,
};
use serde_json::Value;
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

        let bundle = self.get_locale(&lang_arg).ok_or(tera::Error::msg(format!(
            "locale not registered: {lang_arg}"
        )))?;

        let msg = bundle
            .get_message(ftl_key)
            .ok_or(tera::Error::msg(&format!(
                "FTL key not in locale: {}",
                ftl_key
            )))?;
        let pattern = msg
            .value()
            .ok_or(tera::Error::msg("No value in fluent message"))?;

        let fluent_args: FluentArgs = args
            .iter()
            .filter(|(key, _)| key.as_str() != "lang" && key.as_str() != "key")
            .map(|(key, val)| (key, json_value_to_fluent_value(val, self.number_options())))
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

fn json_value_to_fluent_value<'a>(
    json_value: &'a serde_json::Value,
    number_opts: &FluentNumberOptions,
) -> fluent::FluentValue<'a> {
    match json_value {
        Value::Number(n) => n
            .as_f64()
            .map(|n_f64| {
                let f_n = FluentNumber::new(n_f64, number_opts.clone());
                FluentValue::Number(f_n)
            })
            .unwrap_or_else(|| FluentValue::from(n.to_string())),
        Value::String(s) => FluentValue::String(Cow::Borrowed(s)),
        Value::Null => FluentValue::None,
        _ => FluentValue::from(json_value.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_convert_num_to_fluent_num() {
        let num = serde_json::Value::from(2);

        let fluent_num = json_value_to_fluent_value(&num, &FluentNumberOptions::default());

        assert_eq!(
            fluent_num,
            FluentValue::from(FluentNumber::new(2_f64, FluentNumberOptions::default()))
        );
    }
}
