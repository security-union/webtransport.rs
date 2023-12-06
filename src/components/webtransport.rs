use js_sys::Uint8Array;
use leptos::{
    html::{Input, Textarea},
    *,
};
use leptos_webtransport::{WebTransportService, WebTransportStatus, WebTransportTask};

use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use std::sync::Arc;
use web_sys::SubmitEvent;

pub const ECHO_URL: &str = "https://127.0.0.1:4433/";

#[component]
pub fn WebtransportDemo() -> impl IntoView {
    let (data, set_data) = create_signal(String::new());
    let (url, set_url) = create_signal(ECHO_URL.to_string());
    let url_input_element: NodeRef<Input> = create_node_ref();
    let (connect, set_connect) = create_signal(false);
    let (status, set_status) = create_signal(WebTransportStatus::Closed);
    let (transport, set_transport) = create_signal::<Arc<Option<WebTransportTask>>>(Arc::new(None));
    let datagrams = create_rw_signal(create_signal::<Vec<u8>>(Vec::new()).0);
    let unidirectional_streams = create_rw_signal(create_signal::<Option<_>>(None).0);
    let bidirectional_streams = create_rw_signal(create_signal::<Option<_>>(None).0);

    let on_submit = move |ev: SubmitEvent| {
        // stop the page from reloading!
        ev.prevent_default();

        // here, we'll extract the value from the input
        let value = url_input_element()
            // event handlers can only fire after the view
            // is mounted to the DOM, so the `NodeRef` will be `Some`
            .expect("<input> to exist")
            // `NodeRef` implements `Deref` for the DOM element type
            // this means we can call`HtmlInputElement::value()`
            // to get the current value of the input
            .value();

        let connected = connect.get_untracked();

        if !connected {
            if let Ok(t) = WebTransportService::connect(&value) {
                datagrams.set(t.datagram.clone());
                unidirectional_streams.set(t.unidirectional_stream.clone());
                bidirectional_streams.set(t.bidirectional_stream.clone());
                set_status(t.status.get());
                set_transport(Arc::new(Some(t)));
            }
        } else {
            if let Some(t) = transport.get_untracked().as_ref() {
                t.close();
            }
            set_status(WebTransportStatus::Closed);
            set_transport(Arc::new(None));
        }
        set_connect(!connect.get_untracked());
        set_url(value.clone());
    };
    let text_area_element: NodeRef<Textarea> = create_node_ref();

    let send_data = move |ev: SubmitEvent| {
        // stop the page from reloading!
        ev.prevent_default();
        let value = text_area_element()
            .expect("<textarea> to exist")
            .value();
        set_data(value.clone());
        if let Some(t) = transport.get_untracked().as_ref() {
            let method = ev
                .target()
                .expect("event target")
                .unchecked_into::<web_sys::HtmlFormElement>()
                .elements()
                .named_item("method")
                .expect("method")
                .unchecked_into::<web_sys::HtmlInputElement>()
                .value();
            logging::log!("method: {}", method);

            match method.as_str() {
                "send_datagram" => {
                    WebTransportTask::send_datagram(t.transport.clone(), value.as_bytes().to_vec());
                }
                "send_undirectional_stream" => {
                    WebTransportTask::send_unidirectional_stream(t.transport.clone(), value.as_bytes().to_vec());
                }
                _ => {}
            }
        }
    };

    create_effect(move |_| {
        if let Some(t) = transport.get().as_ref() {
            let status = t.status.get();
            set_status(status.clone());
            match status {
                WebTransportStatus::Closed => {
                    logging::log!("WebTransportStatus Connection closed");
                }
                WebTransportStatus::Connecting => {
                    logging::log!("WebTransportStatus Connecting...");
                }
                WebTransportStatus::Opened => {
                    logging::log!("WebTransportStatus Connection opened");
                }
                WebTransportStatus::Error => {
                    logging::error!("WebTransportStatus Connection error");
                }
            }
        }
    });

    create_effect(move |_| {
        let datagram= datagrams.get().get();
        let s = String::from_utf8(datagram).unwrap();
        logging::log!("Received datagram: {}", s);
    });

    create_effect(move |_| {
        let Some(stream) = unidirectional_streams.get().get() else {
            logging::log!("No unidirectional stream");
            return;
        };
        let reader = stream.get_reader().unchecked_into::<web_sys::ReadableStreamDefaultReader>();
        spawn_local(async move {
            let result = JsFuture::from(reader.read()).await.unwrap();
            let done = js_sys::Reflect::get(&result, &JsValue::from_str("done")).unwrap().as_bool().unwrap();
            let value = js_sys::Reflect::get(&result, &JsValue::from_str("value")).unwrap().unchecked_into::<Uint8Array>();
            if done {
                logging::log!("Unidirectional stream closed");
            }
            let value = js_sys::Uint8Array::new(&value);
            let s = String::from_utf8(value.to_vec()).unwrap();
            logging::log!("Received unidirectional stream: {}", s);
            set_data(s);

        });
    });


    view! {
        <form on:submit=on_submit>
            <input type="text" value=url node_ref=url_input_element/>
            <input
                type="submit"
                value=move || { if connect.get() { "Disconnect" } else { "Connect" } }
            />
        </form>
        <h2>{move || { format!("WebTransport Status: {:?}", status.get()) }}
        </h2>
        <form on:submit=send_data>
            <textarea value=data node_ref=text_area_element></textarea>
            <input type="submit" value="Send Data"/>
            <div>
                <input type="radio" name="method" value="send_datagram" checked=true/>
                <label for="send_datagram">Send Datagram</label>
                <input type="radio" name="method" value="send_undirectional_stream"/>
                <label for="send_undirectional_stream">Send Unidirectional Stream</label>
            </div>
        </form>
        <div>
            <h2>Received Data</h2>
            <div>
                <h3>{move || data.get()}</h3>
            </div>
        </div>
    }
}
