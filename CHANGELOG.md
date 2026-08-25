# Change Log

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased] - yyyy-mm-dd

Here we write upgrading notes for brands. It's a team effort to make them as
straightforward as possible.

### Added

- [PROJECTNAME-XXXX](http://tickets.projectname.com/browse/PROJECTNAME-XXXX)
  MINOR Ticket title goes here.
- [PROJECTNAME-YYYY](http://tickets.projectname.com/browse/PROJECTNAME-YYYY)
  PATCH Ticket title goes here.

### Changed

### Fixed

## [0.6.1] - 2026-08-25

Improve debugging.

### Changed

Improved debug feedback from tera when there is a localization key error.

## [0.6.0] - 2026-08-21

Upgraded to tera v2.

### Changed

Upgraded tera feature to work with tera v2. This is a breaking change with tera v1.

Allows the use of localization function fluent in tera functions without having to pass the lang=lang argument each time.

See the [tera migration guide](https://github.com/Keats/tera/blob/master/MIGRATION.md) for more details

## [0.5.1] - 2025-07-24

### Added

- Ability to use tracing for logs

### Changed

- NA

### Fixed

- NA

## [0.5.0] - 2025-07-08

### Added

### Changed

- Updates dependencies

### Fixed

## [0.3.1] - 2024-07-01

### Added

- ability to use [fluent attributes](https://projectfluent.org/fluent/guide/attributes.html) in tera templates.

### Changed

### Fixed

## [0.3.0] - 2024-06-12

### Added

### Changed

- Use `add_resource_overriding` when adding a bundle so that fallbacks can be used
- BREAKING: Use reference of fluent::FluentArgs in Localizer::format_message

### Fixed
