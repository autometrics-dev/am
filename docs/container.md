# AM container

The `am` binary is primarily designed to be a tool to be run locally, during
development. We do however provide a container that can be used for specific
[use cases](#use-cases), but it does come with [limitations](#limitations) when
running it in a container.

Note: the examples use Docker, but they should also work with Podman.

## Getting started

We publish our container images on [Docker Hub](https://hub.docker.com/r/autometrics/am).
You can pull the `latest` image by running the following:

```
docker pull autometrics/am:latest
```

Then you can invoke any of `am`'s commands after that:

```
docker run -it --rm autometrics/am:latest --version
```

<details>
    <summary>`am proxy`</summary>

    If you want to run `am proxy` in a container then we recommend that you use
    [Docker Hub](https://hub.docker.com/r/autometrics/am-proxy). This container
    comes with a entrypoint already set to the proxy command as well as an
    environment variables that allows `am` to listen on all addresses.
</details>

### Versions

For production environments we recommend that you use a specific version of the
container. This will ensure that any breaking changes won't affect your setup.
For every version of `am`, we publish a specific container (see [Docker Hub](https://hub.docker.com/r/autometrics/am/tags)).

### Local configuration

If you want to use `am start` locally, then you will need to ensure that the
container is able to reach your application and that your browser is able to
reach the container. This can be done by using the `--network=host` with Docker
or Podman:

```
docker run -it --rm --network=host autometrics/am:latest start :3000
```

This configuration will configure Prometheus within the container to monitor
you application that is running outside of Docker on port `3000`. You can
access Explorer by visiting `http://localhost:6789` in your browser.

<details>
    <summary>Advanced, non host network setup</summary>

    Alternatively, the following will not use the host network and instead will
    forward a port of the host to the container (Note that this won't allow
    Prometheus to reach your application running on the host):

    ```
    docker run -it --rm -e LISTEN_ADDRESS=0.0.0.0:6789 -P autometrics/am:latest start example.com:3000
    ```

    The extra argument ensures that the host is able to access `am` within the
    container.
</details>

### Docker Desktop

If you are using Docker desktop for Mac or Windows then you won't need to use
the host network, while still being able to communicate with your application
running on the host. This can be done by using the following endpoint:
`host.docker.internal` and adding the port to it:

```
docker run -it --rm -e LISTEN_ADDRESS=0.0.0.0:6789 -P autometrics/am:latest start host.docker.internal:3000
```

## Use cases

### Running it as a proxy

`am` comes with a command called `proxy`. This will allow you to forward traffic
to a Prometheus instance. This command was specifically intended to be used to
be run in a environment such as Kubernetes.

### Being able to easily remove am

If you want to quickly try out `am` then you can easily run it using Docker or
Podman. Because nothing it installed on your machine and only images and
containers are downloaded, it is easy to remove it again.

Be aware of the [limitations](#limitations) of running `am` in a container.

## Limitations

### Data persistence

If you are running `am start` in a container than it will download the
Prometheus and other components to the file system of the container. This means
that if you remove the container then you will need to download the files again.
The same applies to data produced by Prometheus.

### Networking

In some situations it might become difficult, confusing, or even impossible to
configure `am` to reach your application that you want to monitor. This is
because the container will be isolated in its own network (unless configured
differently) and it might also be caused by the container running within a VM.
