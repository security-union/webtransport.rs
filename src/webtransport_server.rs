use anyhow::{Context, Result};
use bytes::Bytes;
use http::Method;
use quinn::VarInt;
use rustls::{Certificate, PrivateKey};
use sec_http3::sec_http3_quinn as h3_quinn;
use sec_http3::webtransport::server::AcceptedBi;
use sec_http3::webtransport::{server::WebTransportSession, stream};
use sec_http3::{
    error::ErrorLevel,
    ext::Protocol,
    quic::{self, RecvDatagramExt, SendDatagramExt, SendStreamUnframed},
    server::Connection,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::{error, info, trace_span};

#[derive(Debug)]
pub struct WebTransportOpt {
    pub listen: SocketAddr,
    pub certs: Certs,
}

#[derive(Debug, Clone)]
pub struct Certs {
    pub cert: PathBuf,
    pub key: PathBuf,
}

fn get_key_and_cert_chain(certs: Certs) -> anyhow::Result<(PrivateKey, Vec<Certificate>)> {
    let key_path = certs.key;
    let cert_path = certs.cert;
    let key = std::fs::read(&key_path).context("failed to read private key")?;
    let key = if key_path.extension().map_or(false, |x| x == "der") {
        PrivateKey(key)
    } else {
        let pkcs8 = rustls_pemfile::pkcs8_private_keys(&mut &*key)
            .context("malformed PKCS #8 private key")?;
        match pkcs8.into_iter().next() {
            Some(x) => PrivateKey(x),
            None => {
                let rsa = rustls_pemfile::rsa_private_keys(&mut &*key)
                    .context("malformed PKCS #1 private key")?;
                match rsa.into_iter().next() {
                    Some(x) => PrivateKey(x),
                    None => {
                        anyhow::bail!("no private keys found");
                    }
                }
            }
        }
    };
    let certs = std::fs::read(&cert_path).context("failed to read certificate chain")?;
    let certs = if cert_path.extension().map_or(false, |x| x == "der") {
        vec![Certificate(certs)]
    } else {
        rustls_pemfile::certs(&mut &*certs)
            .context("invalid PEM-encoded certificate")?
            .into_iter()
            .map(Certificate)
            .collect()
    };
    Ok((key, certs))
}

pub async fn start(opt: WebTransportOpt) -> Result<(), Box<dyn std::error::Error>> {
    info!("WebTransportOpt: {opt:#?}");

    let (key, certs) = get_key_and_cert_chain(opt.certs)?;

    let mut tls_config = rustls::ServerConfig::builder()
        .with_safe_default_cipher_suites()
        .with_safe_default_kx_groups()
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    tls_config.max_early_data_size = u32::MAX;
    let alpn: Vec<Vec<u8>> = vec![
        b"h3".to_vec(),
        b"h3-32".to_vec(),
        b"h3-31".to_vec(),
        b"h3-30".to_vec(),
        b"h3-29".to_vec(),
    ];
    tls_config.alpn_protocols = alpn;

    // 1. create quinn server endpoint and bind UDP socket
    let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(tls_config));
    let mut transport_config = quinn::TransportConfig::default();
    transport_config.keep_alive_interval(Some(Duration::from_secs(2)));
    transport_config.max_idle_timeout(Some(VarInt::from_u32(10_000).into()));
    server_config.transport = Arc::new(transport_config);
    let endpoint = quinn::Endpoint::server(server_config, opt.listen)?;

    info!("listening on {}", opt.listen);

    // 2. Accept new quic connections and spawn a new task to handle them
    while let Some(new_conn) = endpoint.accept().await {
        trace_span!("New connection being attempted");
        tokio::spawn(async move {
            match new_conn.await {
                Ok(conn) => {
                    info!("new http3 established");
                    let h3_conn = sec_http3::server::builder()
                        .enable_webtransport(true)
                        .enable_connect(true)
                        .enable_datagram(true)
                        .max_webtransport_sessions(1)
                        .send_grease(true)
                        .build(h3_quinn::Connection::new(conn))
                        .await
                        .unwrap();

                    if let Err(err) = handle_connection(h3_conn).await {
                        error!("Failed to handle connection: {err:?}");
                    }
                }
                Err(err) => {
                    error!("accepting connection failed: {:?}", err);
                }
            }
        });
    }

    // shut down gracefully
    // wait for connections to be closed before exiting
    endpoint.wait_idle().await;

    Ok(())
}

async fn handle_connection(mut conn: Connection<h3_quinn::Connection, Bytes>) -> Result<()> {
    // 3. TODO: Conditionally, if the client indicated that this is a webtransport session, we should accept it here, else use regular h3.
    // if this is a webtransport session, then h3 needs to stop handing the datagrams, bidirectional streams, and unidirectional streams and give them
    // to the webtransport session.

    loop {
        match conn.accept().await {
            Ok(Some((req, stream))) => {
                info!("new request: {:#?}", req);
                let ext = req.extensions();
                match req.method() {
                    &Method::CONNECT if ext.get::<Protocol>() == Some(&Protocol::WEB_TRANSPORT) => {
                        info!("Handing over connection to WebTransport");
                        let session = WebTransportSession::accept(req, stream, conn).await?;
                        info!("Established webtransport session");
                        // 4. Get datagrams, bidirectional streams, and unidirectional streams and wait for client requests here.
                        // h3_conn needs to handover the datagrams, bidirectional streams, and unidirectional streams to the webtransport session.
                        handle_session(session).await?;
                        return Ok(());
                    }
                    _ => {
                        info!(?req, "Received request");
                    }
                }
            }

            // indicating no more streams to be received
            Ok(None) => {
                break;
            }

            Err(err) => {
                error!("Error on accept {}", err);
                match err.get_error_level() {
                    ErrorLevel::ConnectionError => break,
                    ErrorLevel::StreamError => continue,
                }
            }
        }
    }
    Ok(())
}

#[tracing::instrument(level = "trace", skip(session))]
async fn handle_session<C>(session: WebTransportSession<C, Bytes>) -> anyhow::Result<()>
where
    // Use trait bounds to ensure we only happen to use implementation that are only for the quinn
    // backend.
    C: 'static
        + Send
        + sec_http3::quic::Connection<Bytes>
        + RecvDatagramExt<Buf = Bytes>
        + SendDatagramExt<Bytes>,
    <C::SendStream as sec_http3::quic::SendStream<Bytes>>::Error:
        'static + std::error::Error + Send + Sync + Into<std::io::Error>,
    <C::RecvStream as sec_http3::quic::RecvStream>::Error:
        'static + std::error::Error + Send + Sync + Into<std::io::Error>,
    stream::BidiStream<C::BidiStream, Bytes>:
        quic::BidiStream<Bytes> + Unpin + AsyncWrite + AsyncRead,
    <stream::BidiStream<C::BidiStream, Bytes> as quic::BidiStream<Bytes>>::SendStream:
        Unpin + AsyncWrite + Send + Sync,
    <stream::BidiStream<C::BidiStream, Bytes> as quic::BidiStream<Bytes>>::RecvStream:
        Unpin + AsyncRead + Send + Sync,
    C::SendStream: Send + Sync + Unpin,
    C::RecvStream: Send + Unpin,
    C::BidiStream: Send + Unpin,
    stream::SendStream<C::SendStream, Bytes>: AsyncWrite,
    C::BidiStream: SendStreamUnframed<Bytes>,
    C::SendStream: SendStreamUnframed<Bytes> + Send,
    <C as sec_http3::quic::Connection<bytes::Bytes>>::OpenStreams: Send,
    <C as sec_http3::quic::Connection<bytes::Bytes>>::BidiStream: Sync,
{
    let session_id = session.session_id();
    let should_run = Arc::new(AtomicBool::new(true));
    let s = Arc::new(session);
    info!("WebTransport session established {:?}", session_id);

    let should_run2 = should_run.clone();

    let quic_task = tokio::spawn(async move {
        // let s = s.clone();
        while should_run2.load(Ordering::SeqCst) {
            let session = s.clone();
            tokio::select! {
                datagram = session.accept_datagram() => {
                    if let Ok(Some((_id, buf))) = datagram {
                        info!("Echoing datagram: {:?}", buf);
                        let Ok(_) = session.send_datagram(buf) else {
                            error!("Error sending datagram");
                            return;
                        };
                    } else {
                        error!("Error receiving datagram");
                        return;
                    }
                }
                uni_stream = session.accept_uni() => {
                    if let Ok(Some((_id, mut uni_stream))) = uni_stream {
                        tokio::spawn(async move {
                            let mut buf = Vec::new();
                            let Ok(_n) = uni_stream.read_to_end(&mut buf).await else {
                                error!("Error reading from unidirectional stream");
                                return;
                            };
                            info!("Echoing unidirectional stream data: {:?}", buf);
                            let Ok(mut stream) = session.open_uni(session_id).await else {
                                error!("Error opening unidirectional stream");
                                return;
                            };
                            let Ok(_) = stream.write_all(&buf).await else {
                                error!("Error writing to unidirectional stream");
                                return;
                            };
                        });
                    } else {
                        error!("Error receiving unidirectional stream");
                        return;
                    }
                }
                bidi_stream = session.accept_bi() => {
                    if let Ok(Some(AcceptedBi::Request(_id, mut bidi_stream))) = bidi_stream {
                        tokio::spawn(async move {
                            let Ok(Some(mut buf)) = bidi_stream.recv_data().await else {
                                error!("Error reading from bidirectional stream");
                                return;
                            };
                            info!("Echoing bidirectional stream data");
                            let Ok(mut stream) = session.open_bi(session_id).await else {
                                error!("Error opening bidirectional stream");
                                return;
                            };
                            let Ok(_) = stream.write_all_buf(&mut buf).await else {
                                error!("Error writing to bidirectional stream");
                                return;
                            };
                        });
                    } else {
                        error!("Error receiving bidirectional stream");
                        return;
                    }
                }
            }
        }
    });

    quic_task.await?;
    should_run.store(false, Ordering::SeqCst);
    info!("Finished handling session");
    Ok(())
}
