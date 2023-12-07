FROM rust:1.74.0-slim-bullseye

RUN rustup install nightly-2023-11-23 && \
    rustup default nightly-2023-11-23 && \
    rustup target add wasm32-unknown-unknown

RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev

RUN cargo install cargo-leptos
