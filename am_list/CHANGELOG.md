# Changelog

All notable changes of `am_list` up to 0.3.0 will be documented in this file.
After `0.3.0`, [`am_list`](https://github.com/autometrics-dev/am_list) is
included in [`am`](https://github.com/autometrics-dev/am), and the relevant
changes for `am_list` will be included in `am` README.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Version 0.3.0] - 2023-08-29

### Changed

- Renamed `ExpectedAmLabel` to `FunctionInfo`
- `ExpectedAmLabel.module` and `ExpectedAmLabel.function` are now nested in
  a new `FunctionId` structure: `FunctionInfo.id.(module|function)`.

### Added

- Added locus information in the `FunctionInfo` struct.
  + `list --all` now includes the instrumentation location information when
     available in each returned `FunctionInfo` structure

## [Version 0.2.7] - 2023-08-16

### Changed

- [Typescript] `am_list` will now skip all `node_modules` directory for
  Typescript.
- [Typescript] Include Javascript files in the search

### Fixed

- [Typescript] Fix detection of `@autometrics/autometrics` imports

## [Version 0.2.6] - 2023-07-27

### Added

- [Python] Support for python language

## [Version 0.2.5] - 2023-07-19

### Added

- [All] The `ExpectedAmLabel` structure now implements `serde::Deserialize`

## [Version 0.2.4] - 2023-07-06

### Fixed

- [Go] The `list` subcommand now can also list all functions in a
  project.

## [Version 0.2.3] - 2023-07-04

### Added

- [Typescript] Support for typescript language

## [Version 0.2.2] - 2023-06-19

### Added

- [Rust] The `list` subcommand now takes an optional `--all-functions` (short `-a`) flag,
  which lists all functions/methods in the project instead of only the ones with the
  autometrics annotation. This allows to get an overview of how many functions are
  autometricized. The flag will crash on non-Rust implementations for now.

## [Version 0.2.1] - 2023-06-16

### Fixed

- [Rust] The struct name is now part of the module path for detected methods
- [Rust] Modules defined within a source file are properly detected, and part
  of the module path for detected methods

## [Version 0.2.0] – 2023-06-07

### Added

### Changed

- The command to list all the function names is now a subcommand called 'list'. The
  change is done to accomodate for different subcommands in the future.
- The output of the `list` command is now in JSON, to ease consumption for other
  programs

### Deprecated

### Removed

### Fixed

### Security

## [Version 0.1.0] – 2023-05-29

### Added

- Support for parsing Rust and Go projects

### Changed

### Deprecated

### Removed

### Fixed

### Security

[Version 0.3.0]: https://github.com/autometrics-dev/am_list/compare/v0.2.7...v0.3.0
[Version 0.2.7]: https://github.com/autometrics-dev/am_list/compare/v0.2.6...v0.2.7
[Version 0.2.6]: https://github.com/autometrics-dev/am_list/compare/v0.2.5...v0.2.6
[Version 0.2.5]: https://github.com/autometrics-dev/am_list/compare/v0.2.4...v0.2.5
[Version 0.2.4]: https://github.com/autometrics-dev/am_list/compare/v0.2.3...v0.2.4
[Version 0.2.3]: https://github.com/autometrics-dev/am_list/compare/v0.2.2...v0.2.3
[Version 0.2.2]: https://github.com/autometrics-dev/am_list/compare/v0.2.1...v0.2.2
[Version 0.2.1]: https://github.com/autometrics-dev/am_list/compare/v0.2.0...v0.2.1
[Version 0.2.0]: https://github.com/autometrics-dev/am_list/compare/v0.1.0...v0.2.0
[Version 0.1.0]: https://github.com/autometrics-dev/am_list/releases/tag/v0.1.0
