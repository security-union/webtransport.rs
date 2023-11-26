use leptos::*;
use leptos_webtransport::{WebTransportStatus, WebTransportTask, WebTransportService};
use std::sync::Arc;

const ECHO_URL: &str = "https://echo.rustlemania.com/";

#[component]
pub fn WebtransportDemo() -> impl IntoView {
    let transport = WebTransportService::connect(ECHO_URL).unwrap();

    view! {
        // <button on:click=connect_webtransport>Connect WebTransport</button>
    }

}