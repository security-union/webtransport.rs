
docker run \
    -p 8080:8080 \
    -p 4433:4433/udp \
    -e RUST_LOG=info,quinn=warn,leptos_meta=warn,leptos_router=warn,leptos_dom=warn \
    -e CERT_PATH=/app/certs/localhost.pem \
    -e KEY_PATH=/app/certs/localhost.key \
    -v ./certs:/app/certs \
    securityunion/webtransport-leptos