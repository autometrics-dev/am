# am

`am` is the autometrics companion command line interface (CLI). It makes it easier to create a
local Prometheus environment and inspect the metrics using the explorer.

![The Autometrics Explorer](./assets/am-explorer.png)

## Features

- Download, configure and start various Prometheus components such as,
    - Prometheus - this will scrape, store and expose the metrics data
    - Pushgateway - allow for pushing metrics from batch jobs or short-lived
      processes
    - Grafana (coming soon!)
- Visualize your metrics using the explorer
- Inspect your Service Level Objectives (coming soon!)

## Getting started

### Installation

The recommended installation for macOS is via [Homebrew](https://brew.sh/):

```
brew install autometrics-dev/am
```

Alternatively, you can download the latest version from the [releases page](https://github.com/autometrics-dev/am/releases)

### Quickstart


The following will download, configure and start Prometheus. Assuming you've created an application that is running locally on port `3000` it will start scraping the metrics for that service on that port:

```
am start :3000
```

You can also specify a host, scheme or a path:

```
am start https://example.com:3000/api/metrics
```

It is also possible to specify multiple endpoints:

```
am start :3000 :3030
```

Now you can visualize and inspect your metrics using the explorer by visiting `http://localhost:6789/`.

![The Autometrics Explorer](./assets/explorer.png)

## Documentation

Visit the autometrics docs site for more details on how to use `am` and more
details about autometrics: https://docs.autometrics.dev/

## Contributing

Issues, feature suggestions, and pull requests are very welcome!

If you are interested in getting involved:
- Join the conversation on [Discord](https://discord.gg/9eqGEs56UB)
- Ask questions and share ideas in the [Github Discussions](https://github.com/orgs/autometrics-dev/discussions)
- Take a look at the overall [Autometrics Project Roadmap](https://github.com/orgs/autometrics-dev/projects/1)

## License

`am` is distributed under the terms of both the MIT license and the Apache. See
[LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
