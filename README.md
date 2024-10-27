# webtransport.rs - A Leptos Starter Template
> Live demo available at [webtransport.rs](https://webtransport.rs)

This is a template for use with the [Leptos](https://github.com/leptos-rs/leptos) web framework and the [cargo-leptos](https://github.com/akesson/cargo-leptos) tool.

## Running your project

`cargo leptos watch`  

Close all Chrome instances, then launch chrome using:

```
launch_chrome.sh
```
this will ensure that the Chrome browser trusts the self-signed certificate used by the server.

head to https://127.0.0.1:3000

replace the server endpoint to https://127.0.0.1:3000 to test the WebTransport API.

## Installing Additional Tools

By default, `cargo-leptos` uses `nightly` Rust, `cargo-generate`, and `sass`. If you run into any trouble, you may need to install one or more of these tools.

1. `cargo install --locked cargo-leptos  --version 0.1.11`
1. `rustup default nightly-2023-10-26` - make sure you have Rust nightly
2. `rustup target add wasm32-unknown-unknown` - add the ability to compile Rust to WebAssembly
3. `cargo install cargo-generate` - install `cargo-generate` binary (should be installed automatically in future)
4. `npm install -g sass` - install `dart-sass` (should be optional in future)
5. Install the same exact wasm-bindgen that this project uses: `cargo install -f wasm-bindgen-cli --version 0.2.93`

## Notes about CSR and Trunk:
Although it is not recommended, you can also run your project without server integration using the feature `csr` and `trunk serve`:

`trunk serve --open --features csr`

This may be useful for integrating external tools which require a static site, e.g. `tauri`.
