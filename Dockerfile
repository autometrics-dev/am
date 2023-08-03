# Setting ARCH and TARGET allows us to use different base images for different architectures as well as different targets for rustc
ARG ARCH
ARG TARGET

# builder
FROM ${ARCH}rust:slim-buster AS builder

COPY . .
RUN cargo build --release

# runtime
FROM ${ARCH}debian:buster-slim

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates extra-runtime-dependencies \
 && apt-get clean \
 && apt-get autoremove \
 && rm -rf /var/lib/apt/lists/*

COPY --from=builder target/${TARGET}/release/am /app/

ENTRYPOINT ["/app/am"]
