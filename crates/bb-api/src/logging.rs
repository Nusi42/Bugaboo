//! One log line per request, with a request ID and no personal data (OPS-08, PRIV-01).
//!
//! A line holds: request ID, method, route pattern, status, duration, and the client address
//! cut short (IPv4 `/24`, IPv6 `/48`). It never holds the path as typed, the query, headers or
//! bodies: these can carry names, e-mail addresses or free text. DATA-LOGS guards the code.

use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::extract::connect_info::ConnectInfo;
use axum::extract::{MatchedPath, Request, State};
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use tracing::Instrument as _;

use crate::listener::ClientAddr;

/// Returned on every response, so a person can quote it when asking for help.
pub(crate) const REQUEST_ID_HEADER: &str = "x-request-id";
const FORWARDED_FOR: &str = "x-forwarded-for";
/// Logged for requests that no route pattern matched (files of the browser app, unknown paths).
const NO_ROUTE: &str = "-";

/// What the request log needs to know about the deployment.
#[derive(Clone)]
pub(crate) struct LogContext {
    /// Proxies whose `X-Forwarded-For` is believed.
    pub(crate) trusted_proxies: Vec<IpAddr>,
    started: u64,
    counter: &'static AtomicU64,
}

static REQUESTS: AtomicU64 = AtomicU64::new(0);

impl LogContext {
    pub(crate) fn new(trusted_proxies: Vec<IpAddr>) -> Self {
        let started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        Self {
            trusted_proxies,
            started,
            counter: &REQUESTS,
        }
    }

    /// Unique for this process, and different after a restart. Never taken from the client.
    fn next_id(&self) -> String {
        let count = self.counter.fetch_add(1, Ordering::Relaxed);
        format!("{:08x}-{count:08x}", self.started & 0xffff_ffff)
    }
}

/// The middleware: gives the request an ID, logs the finished request, sets the ID header.
pub(crate) async fn log_request(
    State(context): State<LogContext>,
    request: Request,
    next: Next,
) -> Response {
    let id = context.next_id();
    let method = request.method().clone();
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or(NO_ROUTE, MatchedPath::as_str)
        .to_owned();
    let client = client_of(&request, &context.trusted_proxies).map(truncate);
    let span = tracing::info_span!("request", request_id = %id);
    let started = Instant::now();
    let mut response = next.run(request).instrument(span.clone()).await;
    let _entered = span.enter();
    tracing::info!(
        method = %method,
        route = %route,
        status = response.status().as_u16(),
        duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        client = %client.as_deref().unwrap_or(NO_ROUTE),
        "request"
    );
    if let Ok(value) = HeaderValue::from_str(&id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(REQUEST_ID_HEADER), value);
    }
    response
}

/// Who is on the other end: the connected address, or for a trusted proxy the last address it
/// reports in `X-Forwarded-For` (the one the proxy itself saw).
fn client_of(request: &Request, trusted: &[IpAddr]) -> Option<IpAddr> {
    let peer = request
        .extensions()
        .get::<ConnectInfo<ClientAddr>>()
        .map(|ConnectInfo(ClientAddr(address))| address.ip())?;
    if !trusted.contains(&peer) {
        return Some(peer);
    }
    let forwarded = request
        .headers()
        .get(FORWARDED_FOR)
        .and_then(|value| value.to_str().ok())
        .and_then(|list| list.rsplit(',').next())
        .and_then(|last| last.trim().parse::<IpAddr>().ok());
    Some(forwarded.unwrap_or(peer))
}

/// IPv4 keeps three bytes, IPv6 the first 48 bits; the rest is replaced by zeros.
pub(crate) fn truncate(address: IpAddr) -> String {
    match address {
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
    use super::*;

    #[test]
    fn ops_08_addresses_are_cut_short() {
        assert_eq!(truncate("203.0.113.77".parse().unwrap()), "203.0.113.0");
        assert_eq!(
            truncate("2001:db8:abcd:1234:5678::9".parse().unwrap()),
            "2001:db8:abcd::"
        );
    }

    #[test]
    fn ops_08_request_ids_are_unique() {
        let context = LogContext::new(vec![]);
        assert_ne!(context.next_id(), context.next_id());
    }
}
