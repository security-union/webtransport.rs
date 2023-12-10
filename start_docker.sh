
docker run \
    -p 443:443 \
    -p 443:443/udp \
    -e RUST_LOG=info,quinn=warn,leptos_meta=warn,leptos_router=warn,leptos_dom=warn \
    -e LISTEN_URL=0.0.0.0:443 \
    -e LEPTOS_SITE_ADDR=0.0.0.0:443 \
    -e CERT_PATH=/app/certs/localhost.pem \
    -e KEY_PATH=/app/certs/localhost.key \
    -v ./certs:/app/certs \
    securityunion/webtransport-leptos