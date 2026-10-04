//! Accepting connections: plain or HTTPS, optionally only from trusted proxies (SEC-10).
//!
//! The TLS handshake runs in its own task with a time limit, so one slow or silent client cannot
//! hold up everyone else; the number of handshakes in progress is limited as well.

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use axum::extract::connect_info::Connected;
use axum::serve::{IncomingStream, Listener};
use rustls::ServerConfig;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, mpsc};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;

/// A client that does not finish the handshake in this time is dropped.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_HANDSHAKES: usize = 256;
/// Connections that finished their handshake and wait for the HTTP server.
const READY_QUEUE: usize = 64;
/// Pause after a failed accept (for example too many open files) instead of spinning.
const ACCEPT_RETRY: Duration = Duration::from_millis(100);

/// How the listener treats connections.
#[derive(Clone, Default)]
pub struct ListenerOptions {
    /// Present: every connection is HTTPS with these settings. Absent: plain HTTP.
    pub tls: Option<Arc<ServerConfig>>,
    /// Present: only these addresses may connect (the trusted proxies of a plain listener).
    pub only_from: Option<Vec<IpAddr>>,
}

/// The address a connection came from (what is directly connected, not what a header claims).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientAddr(pub SocketAddr);

/// A connection, plain or encrypted.
pub enum Connection {
    /// Plain HTTP.
    Plain(TcpStream),
    /// HTTPS.
    Tls(Box<TlsStream<TcpStream>>),
}

impl AsyncRead for Connection {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            Self::Tls(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Connection {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            Self::Tls(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_flush(cx),
            Self::Tls(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            Self::Tls(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

/// Connections that are ready for the HTTP server.
pub struct Incoming {
    ready: mpsc::Receiver<(Connection, SocketAddr)>,
    local: SocketAddr,
}

impl Incoming {
    /// Starts accepting on `tcp`. Must be called inside a Tokio runtime.
    ///
    /// # Errors
    /// Returns the error if the local address of `tcp` cannot be read.
    pub fn start(tcp: TcpListener, options: ListenerOptions) -> io::Result<Self> {
        let local = tcp.local_addr()?;
        let (sender, ready) = mpsc::channel(READY_QUEUE);
        tokio::spawn(accept_loop(tcp, options, sender));
        Ok(Self { ready, local })
    }
}

async fn accept_loop(
    tcp: TcpListener,
    options: ListenerOptions,
    sender: mpsc::Sender<(Connection, SocketAddr)>,
) {
    let acceptor = options.tls.map(TlsAcceptor::from);
    let handshakes = Arc::new(Semaphore::new(MAX_HANDSHAKES));
    loop {
        // Waiting for a free handshake slot first slows accepting down when overloaded.
        let Ok(slot) = Arc::clone(&handshakes).acquire_owned().await else {
            return;
        };
        let accepted = tokio::select! {
            () = sender.closed() => return,
            accepted = tcp.accept() => accepted,
        };
        let (stream, peer) = match accepted {
            Ok(connection) => connection,
            Err(error) => {
                tracing::warn!(error = %error, "accepting a connection failed");
                tokio::time::sleep(ACCEPT_RETRY).await;
                continue;
            }
        };
        if let Some(allowed) = &options.only_from
            && !allowed.contains(&peer.ip())
        {
            tracing::debug!("connection from an address that is not a trusted proxy was closed");
            continue;
        }
        let sender = sender.clone();
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            let connection = match acceptor {
                None => Connection::Plain(stream),
                Some(acceptor) => {
                    match tokio::time::timeout(HANDSHAKE_TIMEOUT, acceptor.accept(stream)).await {
                        Ok(Ok(stream)) => Connection::Tls(Box::new(stream)),
                        Ok(Err(error)) => {
                            tracing::debug!(error = %error, "HTTPS handshake failed");
                            return;
                        }
                        Err(_) => {
                            tracing::debug!("HTTPS handshake took too long");
                            return;
                        }
                    }
                }
            };
            drop(slot);
            // Fails only when the server is stopping.
            let _ = sender.send((connection, peer)).await;
        });
    }
}

impl Listener for Incoming {
    type Io = Connection;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            if let Some(connection) = self.ready.recv().await {
                return connection;
            }
            // The accept task ended; nothing will ever come, so wait for the stop signal.
            std::future::pending::<()>().await;
        }
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        Ok(self.local)
    }
}

impl Connected<IncomingStream<'_, Incoming>> for ClientAddr {
    fn connect_info(stream: IncomingStream<'_, Incoming>) -> Self {
        Self(*stream.remote_addr())
    }
}
