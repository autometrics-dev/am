# Release process

In order to release a new version of `am`, first **update the `version` within `Cargo.toml`** to the next desired version,
taking into account any [Semver](https://semver.org/) version bumps.

Once your PR is approved and merged, create a **GitHub release** with the same `tag` as you set `version` to in `Cargo.toml`.
Our release GitHub actions workflow will automatically build the binaries and attach them to the release.
