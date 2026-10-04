//! Rate limits (SEC-14, architecture §7 "Limits and abuse").
//!
//! Three limits, all counted per minute, all kept in memory (a restart forgets them, which only
//! ever lets a client try again sooner than a minute):
//!
//! - every request to `/api` counts against its **client network**;
//! - sign-in attempts count against the client network too, with a much smaller limit
//!   ([`limit_sign_in`], put on the sign-in routes);
//! - sign-in attempts also count against the **account** they try ([`AccountLimit`]), so many
//!   addresses cannot take turns on one account. The account is kept only as a keyed hash of
//!   the lower-case name, never as the name.
//!
//! The client network is the address [`crate::logging`] works out from the connection and, for a
//! trusted proxy, its `X-Forwarded-For` entry (never the raw header). IPv6 clients are counted
//! by their `/64`, because one machine owns that many addresses. Going over a limit answers
//! `429` with `Retry-After` and the message `error.rate-limit.exceeded`; it never locks anything
//! for longer than the minute (an attacker cannot lock a person out for good).
//!
//! Own code instead of `tower_governor`: the fixed window below is thirty lines, needs no other
//! dependency, reads the canonical client address that the logging layer already works out, and
//! has the account hook sign-in needs, which `tower_governor`'s one-key-per-layer design does not
//! give. A fixed window allows up to twice the limit around a window edge; that is accepted for
//! these limits.

use std::collections::HashMap;
use std::hash::{BuildHasher as _, Hash, RandomState};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::extract::{FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::{IntoResponse as _, Response};
use bb_core::ErrorCode;

use crate::errors::ApiError;
use crate::logging::ClientIp;

/// How long a window lasts.
const PERIOD: Duration = Duration::from_mins(1);

/// Each window remembers at most this many clients. A flood of invented addresses must not be
/// able to use up the memory; when the table is full and nothing has expired, it starts over.
const MAX_CLIENTS: usize = 100_000;

/// Requests of the whole API, per client network and minute, when nothing else is set.
pub const DEFAULT_REQUESTS_PER_MINUTE: u32 = 600;

/// Sign-in attempts per client network and per account and minute, when nothing else is set.
pub const DEFAULT_SIGN_IN_ATTEMPTS_PER_MINUTE: u32 = 10;

/// The limits, from the start-up settings (OPS-10; they move to the settings registry later).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LimitsConfig {
    /// Requests to `/api` per client network and minute.
    pub requests_per_minute: u32,
    /// Sign-in attempts per client network and per account and minute.
    pub sign_in_attempts_per_minute: u32,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: DEFAULT_REQUESTS_PER_MINUTE,
            sign_in_attempts_per_minute: DEFAULT_SIGN_IN_ATTEMPTS_PER_MINUTE,
        }
    }
}

struct Entry {
    started: Instant,
    count: u32,
}

/// Counts events per key in fixed windows.
struct Window<K> {
    limit: u32,
    entries: Mutex<HashMap<K, Entry>>,
}

impl<K: Hash + Eq> Window<K> {
    fn new(limit: u32) -> Self {
        Self {
            limit,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Counts one event. `Err` holds the seconds to wait (at least one) when the key is over
    /// the limit.
    fn hit(&self, key: K, now: Instant) -> Result<(), u64> {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        if entries.len() >= MAX_CLIENTS && !entries.contains_key(&key) {
            entries.retain(|_, entry| now.duration_since(entry.started) < PERIOD);
            if entries.len() >= MAX_CLIENTS {
                entries.clear();
            }
        }
        let entry = entries.entry(key).or_insert(Entry {
            started: now,
            count: 0,
        });
        if now.duration_since(entry.started) >= PERIOD {
            *entry = Entry {
                started: now,
                count: 0,
            };
        }
        if entry.count >= self.limit {
            let wait = PERIOD.saturating_sub(now.duration_since(entry.started));
            let seconds = wait.as_secs() + u64::from(wait.subsec_nanos() > 0);
            return Err(seconds.max(1));
        }
        entry.count += 1;
        Ok(())
    }
}

/// All limits of the server.
pub struct Limits {
    requests: Window<IpAddr>,
    sign_in_networks: Window<IpAddr>,
    accounts: Window<u64>,
    /// Random for each start, so the hash of an account name cannot be computed elsewhere.
    hasher: RandomState,
}

impl Limits {
    /// The limits with the given numbers. A limit of `0` refuses everything.
    #[must_use]
    pub fn new(config: LimitsConfig) -> Arc<Self> {
        Arc::new(Self {
            requests: Window::new(config.requests_per_minute),
            sign_in_networks: Window::new(config.sign_in_attempts_per_minute),
            accounts: Window::new(config.sign_in_attempts_per_minute),
            hasher: RandomState::new(),
        })
    }

    /// Counts one request of `client` against the limit of the whole API.
    ///
    /// # Errors
    /// `429` with `Retry-After` when the client network is over its limit.
    pub fn check_request(&self, client: IpAddr) -> Result<(), ApiError> {
        self.check_request_at(client, Instant::now())
    }

    fn check_request_at(&self, client: IpAddr, now: Instant) -> Result<(), ApiError> {
        self.requests.hit(network_of(client), now).map_err(too_many)
    }

    /// Counts one sign-in attempt of `client`.
    ///
    /// # Errors
    /// `429` with `Retry-After` when the client network is over its limit.
    pub fn check_sign_in(&self, client: IpAddr) -> Result<(), ApiError> {
        self.check_sign_in_at(client, Instant::now())
    }

    fn check_sign_in_at(&self, client: IpAddr, now: Instant) -> Result<(), ApiError> {
        self.sign_in_networks
            .hit(network_of(client), now)
            .map_err(too_many)
    }

    /// Counts one sign-in attempt against `account` (the e-mail address as typed; spelling in
    /// upper or lower case counts as the same account).
    ///
    /// # Errors
    /// `429` with `Retry-After` when the account is over its limit.
    pub fn check_account(&self, account: &str) -> Result<(), ApiError> {
        self.check_account_at(account, Instant::now())
    }

    fn check_account_at(&self, account: &str, now: Instant) -> Result<(), ApiError> {
        let key = self.hasher.hash_one(account.trim().to_lowercase());
        self.accounts.hit(key, now).map_err(too_many)
    }
}

fn too_many(seconds: u64) -> ApiError {
    ApiError::new(ErrorCode::TooManyRequests).with_retry_after(seconds)
}

/// Clients are counted by network: an IPv6 address by its `/64`, an IPv4 address (also in IPv6
/// form) as it is.
fn network_of(address: IpAddr) -> IpAddr {
    match address.to_canonical() {
        IpAddr::V6(v6) => {
            let [a, b, c, d, ..] = v6.segments();
            IpAddr::V6([a, b, c, d, 0, 0, 0, 0].into())
        }
        v4 @ IpAddr::V4(_) => v4,
    }
}

/// A request without a known client address (never the case behind the real listener) is
/// counted with all others like it, so a wiring mistake limits instead of opening everything.
const UNKNOWN_CLIENT: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);

fn client_of(request: &Request) -> IpAddr {
    request
        .extensions()
        .get::<ClientIp>()
        .map_or(UNKNOWN_CLIENT, |ClientIp(address)| *address)
}

/// The middleware of the whole API: counts a request below `/api` against its client network and
/// hands the limits to the handlers ([`AccountLimit`]). Pages, files and health checks are not
/// counted.
pub(crate) async fn limit_requests(
    State(limits): State<Arc<Limits>>,
    mut request: Request,
    next: Next,
) -> Response {
    if !crate::api::is_api(request.uri().path()) {
        return next.run(request).await;
    }
    if let Err(error) = limits.check_request(client_of(&request)) {
        return error.into_response();
    }
    request.extensions_mut().insert(limits);
    next.run(request).await
}

/// The middleware of the sign-in routes (`route_layer(from_fn_with_state(limits, limit_sign_in))`):
/// counts the attempt against the client network, with the small limit for sign-in.
pub async fn limit_sign_in(
    State(limits): State<Arc<Limits>>,
    request: Request,
    next: Next,
) -> Response {
    if let Err(error) = limits.check_sign_in(client_of(&request)) {
        return error.into_response();
    }
    next.run(request).await
}

/// The per-account limit for a sign-in handler: once the handler knows which account is tried,
/// `account_limit.check(&email)?` counts the attempt.
pub struct AccountLimit(Arc<Limits>);

impl AccountLimit {
    /// Counts one attempt against `account`.
    ///
    /// # Errors
    /// `429` with `Retry-After` when the account is over its limit.
    pub fn check(&self, account: &str) -> Result<(), ApiError> {
        self.0.check_account(account)
    }
}

impl<S: Send + Sync> FromRequestParts<S> for AccountLimit {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, ApiError> {
        parts
            .extensions
            .get::<Arc<Limits>>()
            .cloned()
            .map(Self)
            .ok_or_else(|| ApiError::internal(&"the rate limits are not set up for this route"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(requests: u32, sign_in: u32) -> Arc<Limits> {
        Limits::new(LimitsConfig {
            requests_per_minute: requests,
            sign_in_attempts_per_minute: sign_in,
        })
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    fn wait_of(error: &ApiError) -> u64 {
        let response = error.into_response();
        response.headers()[axum::http::header::RETRY_AFTER]
            .to_str()
            .unwrap()
            .parse()
            .unwrap()
    }

    #[test]
    fn sec_14_a_client_over_the_limit_waits_until_the_window_ends() {
        let limits = limits(3, 3);
        let start = Instant::now();
        let client = ip("203.0.113.7");
        for _ in 0..3 {
            limits.check_request_at(client, start).unwrap();
        }
        let error = limits
            .check_request_at(client, start + Duration::from_secs(20))
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::TooManyRequests);
        assert_eq!(wait_of(&error), 40);
        // After the window the client may try again.
        limits
            .check_request_at(client, start + Duration::from_secs(61))
            .unwrap();
    }

    #[test]
    fn sec_14_clients_do_not_share_a_limit() {
        let limits = limits(1, 1);
        let now = Instant::now();
        limits.check_request_at(ip("203.0.113.7"), now).unwrap();
        limits.check_request_at(ip("203.0.113.8"), now).unwrap();
        assert!(limits.check_request_at(ip("203.0.113.7"), now).is_err());
    }

    #[test]
    fn sec_14_ipv6_clients_are_counted_by_their_64_and_mapped_addresses_as_ipv4() {
        let limits = limits(1, 1);
        let now = Instant::now();
        limits.check_request_at(ip("2001:db8:1:2::1"), now).unwrap();
        assert!(
            limits
                .check_request_at(ip("2001:db8:1:2:ffff::9"), now)
                .is_err()
        );
        limits.check_request_at(ip("2001:db8:1:3::1"), now).unwrap();
        limits
            .check_request_at(ip("::ffff:203.0.113.7"), now)
            .unwrap();
        assert!(limits.check_request_at(ip("203.0.113.7"), now).is_err());
    }

    #[test]
    fn sec_14_sign_in_has_its_own_limits_per_network_and_per_account() {
        let limits = limits(1000, 2);
        let now = Instant::now();
        let client = ip("203.0.113.7");
        limits.check_sign_in_at(client, now).unwrap();
        limits.check_sign_in_at(client, now).unwrap();
        assert!(limits.check_sign_in_at(client, now).is_err());
        // The general limit is not used up by sign-in.
        limits.check_request_at(client, now).unwrap();
        // Many networks cannot take turns on one account; spelling does not matter.
        limits.check_account_at("Ada@Example.test", now).unwrap();
        limits.check_account_at(" ada@example.test ", now).unwrap();
        let error = limits
            .check_account_at("ADA@example.test", now)
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::TooManyRequests);
        limits.check_account_at("bob@example.test", now).unwrap();
    }

    #[test]
    fn sec_14_a_limit_of_zero_refuses_everything_and_the_wait_is_at_least_a_second() {
        let limits = limits(0, 0);
        let now = Instant::now();
        let error = limits.check_request_at(ip("203.0.113.7"), now).unwrap_err();
        assert_eq!(wait_of(&error), 60);
        let later = now + Duration::from_millis(59_900);
        let _ = limits.check_request_at(ip("203.0.113.7"), later);
        // A fresh key that only just started: still rounds up, never to zero.
        let error = limits
            .check_request_at(ip("203.0.113.8"), later)
            .unwrap_err();
        assert!(wait_of(&error) >= 1);
    }

    #[test]
    fn sec_14_a_flood_of_invented_clients_cannot_grow_the_table_without_end() {
        let window: Window<u32> = Window::new(1);
        let now = Instant::now();
        for key in 0..u32::try_from(MAX_CLIENTS + 10).unwrap() {
            let _ = window.hit(key, now);
        }
        let size = window.entries.lock().unwrap().len();
        assert!(size <= MAX_CLIENTS, "{size}");
    }
}
