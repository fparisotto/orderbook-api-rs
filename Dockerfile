# builder
FROM rust:1.66.0-bullseye as builder

# create a new empty shell project
RUN USER=root cargo new --bin orderbook-api-rs
WORKDIR /orderbook-api-rs

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY ./src ./src
COPY ./migrations ./migrations

# build for release
RUN rm ./target/release/deps/orderbook_api_rs-*
RUN cargo build --release

# runner
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && update-ca-certificates

COPY --from=builder /orderbook-api-rs/target/release/orderbook-api-rs .

EXPOSE 3000

CMD ["./orderbook-api-rs"]
