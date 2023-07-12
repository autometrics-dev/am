# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

## [0.1.0]

- Initial release
- Instead of only copying the prometheus binary, simply extract everything (#17)
- Add more flexible endpoints parser (#21)
- Refactor downloading and verifying Prometheus archive (#32)
