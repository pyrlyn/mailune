# Builds the mailune-server binary and runs that binary.
# Rust 1.99.0 matches mise.toml. The runtime image does not include a toolchain.
FROM rust:1.99.0-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p mailune-server

FROM debian:bookworm-slim
COPY --from=build /src/target/release/mailune-server /usr/local/bin/mailune-server
USER nobody
ENTRYPOINT ["mailune-server"]
