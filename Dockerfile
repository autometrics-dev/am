# Use a Rust image as the base
FROM rust:latest AS builder

# Set the working directory
WORKDIR /app

# Copy the source code into the container
COPY . .

# Build the Rust project
RUN cargo build --release

# Create a new image with only the compiled binary
FROM debian:bookworm-slim

# Set the working directory
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --force-yes --no-install-recommends ca-certificates \
    && apt-get clean \
    && apt-get autoremove \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/am .

# Make the binary executable
RUN chmod +x am

# ARG APP_USER=gravel
# RUN addgroup -S $APP_USER && adduser -S -g $APP_USER $APP_USER
# RUN chown -R $APP_USER:$APP_USER /usr/bin/gravel-gateway
# USER $APP_USER
EXPOSE 6789

ENV PROMETHEUS_URL="http://prom:9090"

ENTRYPOINT [ "./am", "proxy", "--prometheus-url", "${PROMETHEUS_URL}" ]


# Set the entry point for the container
CMD ["./am"]