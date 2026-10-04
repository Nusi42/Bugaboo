//! Accepting connections: plain or HTTPS, optionally only from trusted proxies (SEC-10).
//!
//! The TLS handshake runs in its own task with a time limit, so one slow or silent client cannot
//! hold up everyone else. The number of handshakes in progress and the total number of open
//! connections are limited, so a flood of slow connections cannot use up the machine.

use std::io;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use rustls::ServerConfig;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;

/// A client that does not finish the handshake in this time is dropped.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_HANDSHAKES: usize = 256;
/// Open connections allowed when nothing else is configured.
pub const DEFAULT_MAX_CONNECTIONS: usize = 1024;
/// Connections that finished their handshake and wait for the HTTP server.
const READY_QUEUE: usize = 64;
/// Pause after a failed accept (for example too many open files) instead of spinning.
const ACCEPT_RETRY: Duration = Duration::from_millis(100);
/// A failing accept is logged at most this often.
const ACCEPT_WARNING_EVERY: Duration = Duration::from_secs(10);

/// How the listener treats connections.
#[derive(Clone)]
pub struct ListenerOptions {
    /// Present: every connection is HTTPS with these settings. Absent: plain HTTP.
    pub tls: Option<Arc<ServerConfig>>,
    /// Present: only these addresses may connect (the trusted proxies of a plain listener).
    pub only_from: Option<Vec<IpAddr>>,
    /// The most connections open at the same time; further ones wait in the system's queue.
    pub max_connections: usize,
}

impl Default for ListenerOptions {
    fn default() -> Self {
        Self {
            tls: None,
            only_from: None,
            max_connections: DEFAULT_MAX_CONNECTIONS,
        }
    }
}

/// The address a connection came from (what is directly connected, not what a header claims).
/// IPv4 addresses that arrive in IPv6 form (`::ffff:a.b.c.d`) are always shown as IPv4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientAddr(pub SocketAddr);

/// `address` with an IPv4-mapped IPv6 address turned into plain IPv4.
pub(crate) fn canonical(address: SocketAddr) -> SocketAddr {
    SocketAddr::new(address.ip().to_canonical(), address.port())
}

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

/// A connection that is ready for the HTTP server. Dropping it frees its place in the limit.
pub struct Accepted {
    /// The connection.
    pub io: Connection,
    /// Who is on the other end (canonical form).
    pub peer: SocketAddr,
    /// Holds one of the allowed connections while the connection is open.
    pub permit: OwnedSemaphorePermit,
}

/// Connections that are ready for the HTTP server.
pub struct Incoming {
    ready: mpsc::Receiver<Accepted>,
}

impl Incoming {
    /// Starts accepting on `tcp`. Must be called inside a Tokio runtime.
    #[must_use]
    pub fn start(tcp: TcpListener, options: ListenerOptions) -> Self {
        let (sender, ready) = mpsc::channel(READY_QUEUE);
        tokio::spawn(accept_loop(tcp, options, sender));
        Self { ready }
    }

    /// The next connection, or `None` when accepting has ended.
    pub async fn accept(&mut self) -> Option<Accepted> {
        self.ready.recv().await
    }
}

async fn accept_loop(tcp: TcpListener, options: ListenerOptions, sender: mpsc::Sender<Accepted>) {
    let acceptor = options.tls.map(TlsAcceptor::from);
    let allowed: Option<Vec<IpAddr>> = options
        .only_from
        .map(|list| list.iter().map(IpAddr::to_canonical).collect());
    let handshakes = Arc::new(Semaphore::new(MAX_HANDSHAKES));
    let connections = Arc::new(Semaphore::new(options.max_connections.max(1)));
    let mut last_warning: Option<Instant> = None;
    loop {
        // Waiting for free places first slows accepting down when overloaded.
        let Ok(permit) = Arc::clone(&connections).acquire_owned().await else {
            return;
        };
        let Ok(slot) = Arc::clone(&handshakes).acquire_owned().await else {
            return;
        };
        let accepted = tokio::select! {
            () = sender.closed() => return,
            accepted = tcp.accept() => accepted,
        };
        let (stream, peer) = match accepted {
            Ok((stream, peer)) => (stream, canonical(peer)),
            Err(error) => {
                if last_warning.is_none_or(|at| at.elapsed() >= ACCEPT_WARNING_EVERY) {
                    tracing::warn!(error = %error, "accepting a connection failed");
                    last_warning = Some(Instant::now());
                }
                tokio::time::sleep(ACCEPT_RETRY).await;
                continue;
            }
        };
        if let Some(allowed) = &allowed
            && !allowed.contains(&peer.ip())
        {
            tracing::debug!("connection from an address that is not a trusted proxy was closed");
            continue;
        }
        let sender = sender.clone();
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            let io = match acceptor {
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
            let _ = sender.send(Accepted { io, peer, permit }).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sec_10_mapped_addresses_become_plain_ipv4() {
        let mapped: SocketAddr = "[::ffff:203.0.113.77]:51000".parse().unwrap();
        assert_eq!(canonical(mapped), "203.0.113.77:51000".parse().unwrap());
        let real: SocketAddr = "[2001:db8::1]:1".parse().unwrap();
        assert_eq!(canonical(real), real);
    }
}
