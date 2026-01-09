# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic
Versioning](https://semver.org/spec/v2.0.0.html).

<!-- ## [Unreleased](https://github.com/dpc/pariter/compare/v0.6.0...HEAD) - ReleaseDate -->

<!-- next-url -->

## [0.6.0](https://github.com/dpc/pariter/compare/v0.3.0...v0.6.0) - 2026-09-01

I didn't do a good job of updating the CHANGELOG, sorry.

### Added

- Profiling methods like `profile_egress`, `profile_ingress`, and more
- `readahead_scoped`

## Changed

- Slight APIs changes to improve scoped utilities
- Default thread num to equal num of physical, not virtual, CPU cores

## [0.3.0](https://github.com/dpc/pariter/compare/v0.2.0...v0.3.0) - 2022-01-08

### Added

- `parallel_filter_scoped`
- `parallel_map_scoped`

## [0.2.0](https://github.com/dpc/pariter/compare/v0.1.0...v0.2.0) - 2021-11-24

### Added

- `readahead()`
- `parallel_filter()`

## [0.1.0] - 2021-11-22

- Initial version
