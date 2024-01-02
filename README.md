# About this Crate

This crate offers some localization tools to be used with Axum.

# Basic usage

You can use this crate to extract a [language identifier](https://gist.github.com/eddieoz/63d839c8a20ef508cfa4fa9562632a21) (ex `en-US`) from a request.

If the mode is set to `RedirectMode::NoRedirect`, the Accept-Language header is used to find the users preferred language. If the mode is set to either `RedirectMode::RedirectToFullLocaleSubPath` or `RedirectMode::RedirectToLanguageSubPath`, the user will be redirected to a sub-path based on their Accept-Language headers, to to the default language if not supported.

```rust
use unic_langid::{langid, LanguageIdentifier};
use axum::Extension;

pub const ENGLISH: LanguageIdentifier = langid!("en");
pub const JAPANESE: LanguageIdentifier = langid!("ja");

let router = axum::Router::new()
      .route("/lists", get(|Extension(lang): Extension<LanguageIdentifier>|
        async move {
          Html(format!("Your language is: {}", lang.to_string()))
        }))
      .layer(axum_l18n::LanguageIdentifierExtractorLayer::new(
          ENGLISH,
          vec![ENGLISH, JAPANESE],
          axum_l18n::RedirectMode::NoRedirect,
      ));
```

# Features

## fluent

Enabling fluent allows you to use the fluent Localizer to add bundles for translation.

See [fluent-rs](https://github.com/projectfluent/fluent-rs) for details about fluent and rust.

### Usage

```rust
use unic_langid::{langid, LanguageIdentifier};
use axum_l18n::Localizer;

pub const ENGLISH: LanguageIdentifier = langid!("en");
pub const JAPANESE: LanguageIdentifier = langid!("ja");
let mut localizer = Localizer::new();

localizer
    .add_bundle(JAPANESE, &["locales/ja/main.ftl", "locales/ja/login.ftl"])
    .unwrap();
localizer
    .add_bundle(ENGLISH, &["locales/en/main.ftl", "locales/en/login.ftl"])
    .unwrap();
```

## tera

Enabling the tera feature allows you to use the fluent translations inside tera templates.

See [tera](https://docs.rs/tera/latest/tera/) for more information on tera.

### Usage

Initialization:

```rust
use tera::Tera;

let mut tera = Tera::new("src/views/templates/**/*").expect("tera parsing error");

let mut localizer = Localizer::new();

localizer
    .add_bundle(ENGLISH, &["locales/en/main.ftl", "locales/en/login.ftl"])
    .unwrap();

tera.register_function("fluent", localizer);
```

Axum handler:

```rust
#[derive(Clone)]
struct ViewRouterState {
    pool: mysql_async::Pool,
    tera: Arc<tera::Tera>,
}

async fn lists_view(
    State(state): State<ViewRouterState>,
    Extension(lang): Extension<LanguageIdentifier>,
) -> axum::response::Response {
    let lists: Vec<String> = List::paginate(&state.pool, claim.sub)
        .await.unwrap();

    let mut ctx = Context::new();
    ctx.insert("lists", &lists);
    ctx.insert("lang", &lang);

    let html = state.tera.render("lists.html", &ctx).unwrap();

    Html(html).into_response()
}
```

In tera template:

```html
<label for="family-id">{{ fluent(key="list-family", lang=lang) }}</label>
<select name="family-id" id="family-id">
  {% for family in families %}
  <option value="{{ family.family_id }}">{{ family.family_name }}</option>
  {% endfor %}
</select>
```
