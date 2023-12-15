use std::{
    fmt::{self, Formatter},
    rc::Rc,
};

use gloo_console::log;
use js_sys::{JsString, Uint8Array};
use leptos::{html::Input, *};
use leptos_use::use_interval_fn;
use leptos_webtransport::{WebTransportService, WebTransportStatus, WebTransportTask};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::WebTransport;
use web_sys::{Event, SubmitEvent};

pub const ECHO_URL: &str = "https://echo.webtransport.rs";

/// Check if webtransport is available using web-sys
fn is_webtransport_available() -> bool {
    let result = WebTransport::new(ECHO_URL);
    if let Err(e) = result {
        // check if the error is due to WebTransport not being available
        let output = format!("{:?}", e);
        // Create a Formatter
        log!("output ", &output);
        return !output.contains("ReferenceError");
    }
    true
}

#[component]
pub fn WebtransportDemo() -> impl IntoView {
    let (data, set_data) = create_signal(String::new());
    let (url, set_url) = create_signal(ECHO_URL.to_string());
    let url_input_element: NodeRef<Input> = create_node_ref();
    let (connect, set_connect) = create_signal(false);
    let (status, set_status) = create_signal(WebTransportStatus::Closed);
    let (transport, set_transport) = create_signal::<Option<Rc<WebTransportTask>>>(None);
    let datagrams = create_rw_signal(create_signal::<Vec<u8>>(Vec::new()).0);
    let unidirectional_streams = create_rw_signal(create_signal::<Option<_>>(None).0);
    let bidirectional_streams = create_rw_signal(create_signal::<Option<_>>(None).0);
    let (msg_rate, set_msg_rate) = create_signal(1);
    let (msg_size, set_msg_size) = create_signal(1);
    let (payload, set_payload) = create_signal::<Option<(String, String)>>(None);
    let (recv_msg_count, set_recv_msg_count) = create_signal(0);
    let (recv_msg_rate, set_recv_msg_rate) = create_signal(0);
    let (bidi_read, bidi_write_signal) = create_signal::<Vec<u8>>(Vec::new());
    let (webtransport_available, set_is_webtransport_available) = create_signal(false);
    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        batch(move || {
            // stop the page from reloading!

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
                    set_transport(Some(Rc::new(t)));
                }
            } else {
                if let Some(t) = transport.get_untracked().as_ref() {
                    t.close();
                }
                set_status(WebTransportStatus::Closed);
                set_transport(None);
            }
            set_connect(!connect.get_untracked());
            set_url(value.clone());
        });
    };

    create_effect(move |_| {
        set_is_webtransport_available(is_webtransport_available());
    });

    create_effect(move |_| {
        let Some((msg, method)) = payload.get() else {
            return;
        };
        use_interval_fn(
            move || {
                if let Some(t) = transport.get().as_ref() {
                    match method.as_str() {
                        "send_datagram" => {
                            WebTransportTask::send_datagram(
                                t.transport.clone(),
                                msg.as_bytes().to_vec(),
                            );
                        }
                        "send_undirectional_stream" => {
                            WebTransportTask::send_unidirectional_stream(
                                t.transport.clone(),
                                msg.as_bytes().to_vec(),
                            );
                        }
                        "send_bidirectional_stream" => {
                            WebTransportTask::send_bidirectional_stream(
                                t.transport.clone(),
                                msg.as_bytes().to_vec(),
                                bidi_write_signal.clone(),
                            );
                        }
                        _ => {}
                    }
                }
            },
            1000 / msg_rate.get_untracked(),
        );
        use_interval_fn(
            move || {
                let n = recv_msg_count.get_untracked();
                set_recv_msg_rate(n);
                set_recv_msg_count(0);
            },
            1000,
        );
    });

    let send_data = move |ev: SubmitEvent| {
        // stop the page from reloading!
        ev.prevent_default();
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
        // Generate a random string of length `size`
        let msg = (0..msg_size.get_untracked())
            .map(|_| rand::random::<u8>() % 26 + 97)
            .map(char::from)
            .collect::<String>();
        set_payload(Some((msg, method)));
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
        batch(move || {
            let datagram = datagrams.get().get();
            let s = String::from_utf8(datagram).unwrap();
            logging::log!("Received datagram: {}", s);
            // push s to the end of the data
            let next_data = s + &data.get_untracked() + &'\n'.to_string();
            //trim the first 200 chars
            let next_data = next_data.chars().take(200).collect::<String>();
            set_data(next_data);
            set_recv_msg_count(recv_msg_count.get_untracked() + 1);
        });
    });

    // handle bidirectional stream
    create_effect(move |_| {
        // geto the inbound data from bidi_read
        batch(move || {
            let bidi_data = bidi_read.get();
            let s = String::from_utf8(bidi_data).unwrap();
            logging::log!("Received bidi data: {}", s);
            // push s to the end of the data
            let next_data = s + &data.get_untracked() + &'\n'.to_string();
            //trim the first 200 chars
            let next_data = next_data.chars().take(200).collect::<String>();
            set_data(next_data);
            set_recv_msg_count(recv_msg_count.get_untracked() + 1);
        });
    });

    create_effect(move |_| {
        let Some(stream) = unidirectional_streams.get().get() else {
            logging::log!("No unidirectional stream");
            return;
        };
        let reader = stream
            .get_reader()
            .unchecked_into::<web_sys::ReadableStreamDefaultReader>();
        spawn_local(async move {
            let result = JsFuture::from(reader.read()).await.unwrap();
            let done = js_sys::Reflect::get(&result, &JsValue::from_str("done"))
                .unwrap()
                .as_bool()
                .unwrap();
            let value = js_sys::Reflect::get(&result, &JsValue::from_str("value"))
                .unwrap()
                .unchecked_into::<Uint8Array>();
            if done {
                logging::log!("Unidirectional stream closed");
            }
            let value = js_sys::Uint8Array::new(&value);
            let s = String::from_utf8(value.to_vec()).unwrap();
            logging::log!("Received unidirectional stream: {}", s);
            set_data(s);
            set_recv_msg_count(recv_msg_count.get_untracked() + 1);
        });
    });

    let show_webtransport_error = move || {
        if !webtransport_available.get() {
            view! {
                <p>WebTransport is not available in your browser. Please use a browser that supports WebTransport.</p>
                <p>Check <a href="https://caniuse.com/webtransport">caniuse.com</a> for the latest browser support.</p>
            }
        } else {
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
                <div>
                <label for="msg_rate">Message Rate (Hz)</label>
                <input type="text" name="msg_rate" value=msg_rate on:input=move |ev: Event| {
                    let value = ev
                        .target()
                        .expect("event target")
                        .unchecked_into::<web_sys::HtmlInputElement>()
                        .value();
                    if let Ok(value) = value.parse::<u64>() {
                        set_msg_rate(value);
                    }
                }/>
            </div>
                <div>
                <label for="msg_size">Message Size (Bytes)</label>
                <input type="text" name="msg_size" value=msg_size on:input=move |ev: Event| {
                    let value = ev
                        .target()
                        .expect("event target")
                        .unchecked_into::<web_sys::HtmlInputElement>()
                        .value();
                    set_msg_size(value.parse::<usize>().unwrap());
                }/>
                </div>
                <input type="submit" value="Start Sending" disabled=move || !connect.get()/>
                <div>
                    <input type="radio" name="method" value="send_datagram" checked=true/>
                    <label for="send_datagram">Send Datagram</label>
                    <input type="radio" name="method" value="send_undirectional_stream"/>
                    <label for="send_undirectional_stream">Send Unidirectional Stream</label>
                    <input type="radio" name="method" value="send_bidirectional_stream"/>
                    <label for="send_bidirectional_stream">Send Bidirectional Stream</label>
                </div>
            </form>
            <div>
                <h2># of received messages in last second</h2>
                <div>
                    <h3>{move || recv_msg_rate.get()}</h3>
                    <p>Received data: {move || data.get()}</p>
                </div>
            </div>
            }
        }
    };

    view! {
        {show_webtransport_error}
    }
}
