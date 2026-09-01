use std::borrow::Cow;

use crate::{fluent::MessageAttribute, Localizer};
use fluent::{
    types::{FluentNumber, FluentNumberOptions},
    FluentArgs, FluentValue,
};
use unic_langid::LanguageIdentifier;

impl tera::Function<tera::TeraResult<String>> for Localizer {
    fn is_safe(&self) -> bool {
        true
    }

    fn call(&self, kwargs: tera::Kwargs, state: &tera::State) -> tera::TeraResult<String> {
        let lang_arg = kwargs
            .get::<&str>("lang")?
            .and_then(|str| str.parse::<LanguageIdentifier>().ok())
            .or(state
                .get::<String>("lang")?
                .and_then(|str| str.parse::<LanguageIdentifier>().ok()))
            .ok_or(tera::Error::message("missing lang param"))?;

        let ftl_key = kwargs
            .get("key")?
            .ok_or(tera::Error::message("missing ftl key"))?;

        let ftl_attribute = kwargs.get("attribute")?;

        let fluent_args: FluentArgs = kwargs
            .iter()
            .filter(|(key, _)| key.as_str().is_some_and(|k| k != "key"))
            .map(|(key, val)| {
                (
                    key.to_string(),
                    tera_value_to_fluent_value(val, self.number_options()),
                )
            })
            .collect();

        let message = if let Some(ftl_attribute) = ftl_attribute {
            self.format_message_result(
                &lang_arg,
                &MessageAttribute {
                    key: ftl_key,
                    attribute: ftl_attribute,
                },
                Some(&fluent_args),
            )
        } else {
            self.format_message_result(&lang_arg, ftl_key, Some(&fluent_args))
        }
        .map_err(|err| tera::Error::message(err.to_string()))?;

        Ok(message)
    }
}

fn tera_value_to_fluent_value<'a>(
    tera_value: &'a tera::Value,
    number_opts: &FluentNumberOptions,
) -> fluent::FluentValue<'a> {
    let opt_v = match tera_value.kind() {
        tera::value::ValueKind::F64 => tera_value.as_f64().map(|n_f64| {
            let f_n = FluentNumber::new(n_f64, number_opts.clone());

            FluentValue::Number(f_n)
        }),
        tera::value::ValueKind::I128 => tera_value.as_i128().map(|n_f64| {
            let f_n = FluentNumber::new(n_f64 as f64, number_opts.clone());

            FluentValue::Number(f_n)
        }),
        tera::value::ValueKind::I64 => tera_value.as_i64().map(|n_f64| {
            let f_n = FluentNumber::new(n_f64 as f64, number_opts.clone());

            FluentValue::Number(f_n)
        }),
        tera::value::ValueKind::U64 => tera_value.as_u64().map(|n_f64| {
            let f_n = FluentNumber::new(n_f64 as f64, number_opts.clone());

            FluentValue::Number(f_n)
        }),
        tera::value::ValueKind::U128 => tera_value.as_u128().map(|n_f64| {
            let f_n = FluentNumber::new(n_f64 as f64, number_opts.clone());

            FluentValue::Number(f_n)
        }),
        tera::value::ValueKind::String => tera_value
            .as_str()
            .map(|s| FluentValue::String(Cow::Borrowed(s))),
        tera::value::ValueKind::None => Some(FluentValue::None),
        _ => Some(FluentValue::from(tera_value.to_string())),
    };

    opt_v.unwrap_or_else(|| FluentValue::from(tera_value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fluent::tests::{MAIN, SUB},
        tests::ENGLISH,
    };

    #[test]
    fn can_convert_num_to_fluent_num() {
        macro_rules! test_nums {
            ($($num: expr),*) => {
              $(
                let num = tera::Value::from($num);

                let fluent_num = tera_value_to_fluent_value(&num, &FluentNumberOptions::default());

                assert_eq!(
                    fluent_num,
                    FluentValue::from(FluentNumber::new($num as f64, FluentNumberOptions::default()))
                );
              )*
            };
        }

        test_nums!(2.1_f64, 1_i128, 3_i64, 4_u64, 6_u128);
    }

    #[test]
    fn can_convert_tera_s_to_fluent_s() {
        let s = String::from(
            "A system of first order homegeneous differential equation
can be solved by finding the eigenvalues and eigenvectors of said system's matrix",
        );
        let tera_s = tera::Value::from(s.as_str());

        assert_eq!(
            tera_value_to_fluent_value(&tera_s, &FluentNumberOptions::default()),
            FluentValue::String(Cow::Borrowed(s.as_str()))
        );
    }

    #[test]
    fn can_parse_lang_from_args() {
        let mut loc = Localizer::new();
        loc.add_bundle(ENGLISH, &[MAIN, SUB]).unwrap();

        let mut tera = tera::Tera::new();

        tera.register_function("fluent", loc);

        let ctx = tera::Context::new();

        let tera_r = tera
            .render_str(r#"{{ fluent(key="test-key-a", lang="en") }}"#, &ctx, false)
            .unwrap();

        assert_eq!(tera_r, String::from("Hello World"));
    }

    #[test]
    fn can_parse_lang_from_context() {
        let mut loc = Localizer::new();
        loc.add_bundle(ENGLISH, &[MAIN, SUB]).unwrap();

        let mut tera = tera::Tera::new();

        tera.register_function("fluent", loc);

        let mut ctx = tera::Context::new();
        ctx.insert("lang", "en");

        let tera_r = tera
            .render_str(r#"{{ fluent(key="test-key-a") }}"#, &ctx, false)
            .unwrap();

        assert_eq!(tera_r, String::from("Hello World"));
    }
}
