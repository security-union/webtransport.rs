FROM rust:1.74-slim-bullseye as development
RUN apt-get update && apt-get install -y pkg-config libssl-dev
RUN cargo install cargo-watch

FROM development as build
ENV CARGO_TARGET_DIR=/app/actix-api/target
COPY . /app
WORKDIR /app/actix-api

RUN cargo build --release

FROM debian:bullseye-slim as production

COPY --from=build /app/actix-api/target/release/webtransport_echo_server /usr/bin/webtransport_echo_server

STOPSIGNAL SIGINT

CMD [ "webtransport_echo_server" ]
