# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- Proxy static assets for explorer instead of loading them directly from explorer.autometrics.dev. This url is also configurable so it is easier to use explorer hosted on a different URL (#142)
- Add new endpoint `/api/functions` listing all autometrics'd functions in the current
  working directory
- `am list all` now detects Python projects that use either `setuptools`, a
  `pyproject.toml`-compatible solution, or a `requirements.txt` file (#143)
- `am list` now properly ignores the `target/` folder of Rust projects (#143)
- `am list` now tries to track Rust module name better when called inside a
  cargo workspace (#143)
- If a proxied request fails, am will now log more information and using
  different log levels, depending on the status code (#146):
    - 4xx: `debug` (still requires `--verbose` flag)
    - 5xx and connection issues: `warn` (will log by default)
    - Other status codes: `trace`
- `am list` now properly detects methods in Go (#148)

## [0.5.0]

- The Prometheus write endpoint is now enabled by default (#136)
- Include `am_list` crate inside the workspace, and add the function listing feature (#131)

## [0.4.1]

- Change Docker Hub organization to `autometrics` (#133)
- Add documentation about running `am` in a container (#135)
- Publish a container optimized for `am proxy` (#135)

## [0.4.0]

- Dump logs if prometheus or pushgateway return a non 0 exit code (#122)
- Change the default explorer endpoint of when using `am explorer` (#120)
- Update all depdencies (#124)
- Fix multiarch docker image for arm64 users (#125)
- Update markdown reference generator command to disable TOC (#127)
- Add `am proxy` command (#128)
- Update more dependencies (#130)

## [0.3.0]

- SHA-256 checksums are now provided for all artifact downloads (#101)
- Added self updater (#102)
- Use `clap-markdown` fork that enables formatting by display name (#103)
- Correct `web.external-url` will now be passed to Prometheus and Pushgateway
  if a custom one is specified with `--listen-address` (#112)
- The generated Prometheus config now gets stored in a unique, temporary location (#113)
- Added new subcommand `init` to create a config file interactively (#117)
- `am` is now available as a multi-arch Docker container on [Docker Hub](https://hub.docker.com/repository/docker/fiberplane/am/general) (#118)

## [0.2.1]

- Do not crash if no `--config-file` is specified and no `am.toml` is found (#106)

## [0.2.0]

- Make logging less verbose, and introduce a `--verbose` flag to verbose logging (#62)
- Use host and port for job name in Prometheus target list (#66)
- Prometheus/Pushgateway data directory no longer defaults to current working directory (#76)
- `--ephemeral` can now be specified to automatically delete data created by
  Prometheus/Pushgateway after the process exits (#76)
- Added new subcommand `discord` which links to the discord server (#80)
- The `/metrics` endpoint now transparently redirects to `/pushgateway/metrics` if
  Pushgateway is enabled (#81)
- Allow using a config file (am.toml) to set some defaults such as endpoints or
  if pushgateway is enabled (#67)
- `honor_labels` will now be set to `true` for the Pushgateway endpoint
  in the generated Prometheus config, if it is enabled (#69)
- Redirect `/graph` to `/explorer/graph.html` which will load a different JS
  script from explorer (#84)
- Shorthand notion for endpoints defined within the config file (`am.toml`) is now
  allowed (#85)
- Allow user to specify the Prometheus scrape interval (#87)
- Added new subcommand `explore` which opens up explorer in the browser (#89)
- The Autometrics SLO rules will now be automatically loaded into Prometheus if
  `--no-rules` is not specified (#94)

## [0.1.0]

- Initial release
- Instead of only copying the prometheus binary, simply extract everything (#17)
- Add more flexible endpoints parser (#21)
- Refactor downloading and verifying Prometheus archive (#32)
