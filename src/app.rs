use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use leptos_webtransport::{WebTransportStatus, WebTransportTask, WebTransportService};
use web_sys::{WebTransportBidirectionalStream, WebTransportReceiveStream};

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/leptos_start.css"/>

        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <Router>
            <main>
                <Routes>
                    <Route path="" view=HomePage/>
                    <Route path="/*any" view=NotFound/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    // Creates a reactive value to update the button
    let (count, set_count) = create_signal(0);
    let on_click = move |_| set_count.update(|count| *count += 1);
    let datagram_signal = create_rw_signal::<Vec<u8>>(Vec::new());
    let unidirectional_stream_signal = create_rw_signal::<Option<WebTransportReceiveStream>>(None);
    let bidirectional_stream_signal = create_rw_signal::<Option<WebTransportBidirectionalStream>>(None);
    let notification_signal = create_rw_signal::<WebTransportStatus>(WebTransportStatus::Closed);
    let transport = create_rw_signal::<Option<WebTransportTask>>(None);
    let connect_webtransport = move |_| {
        let url = "https://transport.rustlemania.com";
        transport.update(move |x| { *x = Some(WebTransportService::connect(url, datagram_signal, unidirectional_stream_signal,bidirectional_stream_signal, notification_signal).unwrap()) });
    };

    view! {
        <h1>"Welcome to Leptos!"</h1>
        <button on:click=on_click>"Click Me: " {count}</button>
        <button on:click=connect_webtransport>Connect WebTransport</button>
    }
}

/// 404 - Not Found
#[component]
fn NotFound() -> impl IntoView {
    // set an HTTP status code 404
    // this is feature gated because it can only be done during
    // initial server-side rendering
    // if you navigate to the 404 page subsequently, the status
    // code will not be set because there is not a new HTTP request
    // to the server
    #[cfg(feature = "ssr")]
    {
        // this can be done inline because it's synchronous
        // if it were async, we'd use a server function
        let resp = expect_context::<leptos_actix::ResponseOptions>();
        resp.set_status(actix_web::http::StatusCode::NOT_FOUND);
    }

    view! {
        <h1>"Not Found"</h1>
    }
}
