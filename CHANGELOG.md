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

## [0.1.0]

- Initial release
- Instead of only copying the prometheus binary, simply extract everything (#17)
- Add more flexible endpoints parser (#21)
- Refactor downloading and verifying Prometheus archive (#32)
