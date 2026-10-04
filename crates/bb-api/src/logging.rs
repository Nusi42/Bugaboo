//! One log line per request, with a request ID and no personal data (OPS-08, PRIV-01).
//!
//! A line holds: request ID, method, route pattern, status, duration, and the client network
//! (IPv4 `/24`, IPv6 `/48`). It never holds the path as typed, the query, headers or bodies:
//! these can carry names, e-mail addresses or free text. DATA-LOGS guards the code.

use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use axum::extract::connect_info::ConnectInfo;
use axum::extract::{MatchedPath, Request, State};
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use rustls::crypto::ring::default_provider;
use tracing::Instrument as _;

use crate::listener::ClientAddr;

/// Returned on every response, so a person can quote it when asking for help.
pub(crate) const REQUEST_ID_HEADER: &str = "x-request-id";
const FORWARDED_FOR: &str = "x-forwarded-for";
/// Logged for requests that no route pattern matched (files of the browser app, unknown paths).
const NO_ROUTE: &str = "-";
/// Random bytes in a request ID (shown as twice as many hex digits).
const ID_BYTES: usize = 8;

/// What the request log needs to know about the deployment.
#[derive(Clone)]
pub(crate) struct LogContext {
    /// Exactly one trusted hop: proxies whose `X-Forwarded-For` is believed (canonical form).
    trusted_proxies: Vec<IpAddr>,
}

/// Only used if the system's random source fails, so IDs stay unique within the process.
static FALLBACK: AtomicU64 = AtomicU64::new(0);

impl LogContext {
    pub(crate) fn new(trusted_proxies: &[IpAddr]) -> Self {
        Self {
            trusted_proxies: trusted_proxies.iter().map(IpAddr::to_canonical).collect(),
        }
    }
}

/// Random, so IDs cannot be guessed or compared between restarts.
fn request_id() -> String {
    let mut bytes = [0_u8; ID_BYTES];
    if default_provider().secure_random.fill(&mut bytes).is_err() {
        bytes = FALLBACK.fetch_add(1, Ordering::Relaxed).to_be_bytes();
    }
    format!("{:016x}", u64::from_be_bytes(bytes))
}

/// The middleware: gives the request an ID, logs the finished request, sets the ID header.
pub(crate) async fn log_request(
    State(context): State<LogContext>,
    request: Request,
    next: Next,
) -> Response {
    let id = request_id();
    let method = request.method().clone();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or(NO_ROUTE, MatchedPath::as_str)
        .to_owned();
    let network = client_of(&request, &context.trusted_proxies).map(truncate);
    let span = tracing::info_span!("request", request_id = %id);
    let started = Instant::now();
    let mut response = next.run(request).instrument(span.clone()).await;
    let _entered = span.enter();
    tracing::info!(
        method = %method,
        route = %route,
        status = response.status().as_u16(),
        duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        net = %network.as_deref().unwrap_or(NO_ROUTE),
        "request"
    );
    if let Ok(value) = HeaderValue::from_str(&id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(REQUEST_ID_HEADER), value);
    }
    response
}

/// Who is on the other end: the connected address, or for a trusted proxy the address it
/// reports. Exactly one trusted hop is supported: the last entry of the last
/// `X-Forwarded-For` line is what the (only) proxy itself saw; earlier entries are what the
/// client claimed and are ignored.
fn client_of(request: &Request, trusted: &[IpAddr]) -> Option<IpAddr> {
    let peer = request
        .extensions()
        .get::<ConnectInfo<ClientAddr>>()
        .map(|ConnectInfo(ClientAddr(address))| address.ip().to_canonical())?;
    if !trusted.contains(&peer) {
        return Some(peer);
    }
    let forwarded = request
        .headers()
        .get_all(FORWARDED_FOR)
        .iter()
        .next_back()
        .and_then(|line| line.to_str().ok())
        .and_then(|line| line.rsplit(',').next())
        .and_then(|last| last.trim().parse::<IpAddr>().ok())
        .map(|address| address.to_canonical());
    Some(forwarded.unwrap_or(peer))
}

/// IPv4 keeps three bytes, IPv6 the first 48 bits; the rest is replaced by zeros.
pub(crate) fn truncate(address: IpAddr) -> String {
    match address.to_canonical() {
        IpAddr::V4(v4) => {
            let [a, b, c, _] = v4.octets();
            format!("{a}.{b}.{c}.0")
        }
        IpAddr::V6(v6) => {
            let [a, b, c, ..] = v6.segments();
            format!("{a:x}:{b:x}:{c:x}::")
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;

    use super::*;

    fn request(peer: &str, lines: &[&str]) -> Request {
        let mut request = axum::http::Request::get("/");
        for line in lines {
            request = request.header(FORWARDED_FOR, *line);
        }
        let mut request = request.body(Body::empty()).unwrap();
        request
            .extensions_mut()
            .insert(ConnectInfo(ClientAddr(peer.parse().unwrap())));
        request
    }

    #[test]
    fn ops_08_addresses_are_cut_short() {
        assert_eq!(truncate("203.0.113.77".parse().unwrap()), "203.0.113.0");
        assert_eq!(
            truncate("2001:db8:abcd:1234:5678::9".parse().unwrap()),
            "2001:db8:abcd::"
        );
        assert_eq!(
            truncate("::ffff:203.0.113.77".parse().unwrap()),
            "203.0.113.0"
        );
    }

    #[test]
    fn ops_08_request_ids_are_random_and_unique() {
        let first = request_id();
        assert_eq!(first.len(), ID_BYTES * 2);
        assert_ne!(first, request_id());
    }

    #[test]
    fn sec_10_forwarded_for_is_believed_only_from_a_trusted_proxy_and_only_its_last_entry() {
        let trusted = LogContext::new(&[IpAddr::from([10, 0, 0, 5])]).trusted_proxies;
        let believed = |peer: &str, lines: &[&str]| client_of(&request(peer, lines), &trusted);
        // Not a proxy: the header is ignored.
        assert_eq!(
            believed("198.51.100.9:1", &["203.0.113.1"]),
            Some(IpAddr::from([198, 51, 100, 9]))
        );
        // A proxy: the last entry of the last line (the one the proxy added).
        assert_eq!(
            believed("10.0.0.5:1", &["1.1.1.1, 2.2.2.2", "9.9.9.9, 203.0.113.1"]),
            Some(IpAddr::from([203, 0, 113, 1]))
        );
        // A proxy without a usable header: the proxy itself.
        assert_eq!(
            believed("10.0.0.5:1", &["not an address"]),
            Some(IpAddr::from([10, 0, 0, 5]))
        );
        // Mapped forms count as the same address, for the proxy and the client.
        assert_eq!(
            believed("[::ffff:10.0.0.5]:1", &["::ffff:203.0.113.77"]),
            Some(IpAddr::from([203, 0, 113, 77]))
        );
        let mapped_proxy = LogContext::new(&["::ffff:10.0.0.5".parse().unwrap()]).trusted_proxies;
        assert_eq!(
            client_of(&request("10.0.0.5:1", &["203.0.113.2"]), &mapped_proxy),
            Some(IpAddr::from([203, 0, 113, 2]))
        );
    }
}
