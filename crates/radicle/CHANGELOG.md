# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Introduce a node event for canonical reference updates, `Event::CanonicalRefUpdated`.
  Whenever the node fetches new updates, it checks if canonical references can
  be updated. The node has learned how to return these results and emit them as
  node events. This is a breaking change since it adds a new variant the `Event`
  type.
- Add `#[non_exhaustive]` to `Event` to prevent any further breaking changes
  when adding new variants.

### Changed

- `radicle::profile::Home::socket` defaults to the path `\\.\pipe\radicle-node`
  on Windows. The behavior on Unix-like systems has *not* changed.

### Deprecated

### Removed

- `radicle::node::DEFAULT_SOCKET_NAME`, use `radicle::profile::Home::socket`
  instead.

### Security
