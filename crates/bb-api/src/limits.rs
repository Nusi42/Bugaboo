//! Rate limits (SEC-14, architecture §7 "Limits and abuse").
//!
//! Four limits, all counted per minute, all kept in memory (a restart forgets them, which only
//! ever lets a client try again sooner than a minute):
//!
//! - every request to `/api` counts against its **client network**;
//! - every try of the sign-in family counts against the client's **network** too, with a generous
//!   cap against floods ([`limit_sign_in`], put on the sign-in routes);
//! - **failed** tries are limited per account, person and network in `crate::failures`
//!   (`FailureLimits`): only failures count and a success clears them, so nobody can lock another
//!   person out by merely trying their address (M1-F27);
//! - every request that comes with an **access key** counts against that key too
//!   ([`limit_access_keys`]), besides the network, so one script that runs wild uses up its own
//!   key and nobody else's (SEC-14). The key is counted by the hash of what was presented, so
//!   only somebody who holds the key can use up its limit; guessing keys is limited by the failed
//!   tries of the network in `bb-app`.
//!
//! Two sizes of "network" are used, each defined once. The sign-in cap and the failure keys use
//! `bb_core::network::NetworkPrefix` (IPv4 `/24`, IPv6 `/48`: the shortened address the security
//! log also keeps), so both mean the same group of clients. The limit on all requests is more
//! generous and counts per client address (IPv6 by the `/64`, because one machine owns that many
//! addresses). The client address is the one [`crate::logging`] works out from the connection
//! and, for a trusted proxy, its `X-Forwarded-For` entry (never the raw header). Going over a limit answers
//! `429` with `Retry-After` and the message `error.rate-limit.exceeded`; it never locks anything
//! for longer than the minute here (the growing wait for failures is in `crate::failures`).
//!
//! Own code instead of `tower_governor`: the fixed window below is thirty lines, needs no other
//! dependency, reads the canonical client address that the logging layer already works out, and
//! has the account hook sign-in needs, which `tower_governor`'s one-key-per-layer design does not
//! give. A fixed window allows up to twice the limit around a window edge; that is accepted for
//! these limits.

use std::collections::HashMap;
use std::hash::Hash;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse as _, Response};
use bb_core::ErrorCode;
use bb_core::id::UserId;
use bb_core::network::NetworkPrefix;
use bb_core::security_log::Digest;

use crate::errors::ApiError;
use crate::failures::DEFAULT_SIGN_IN_FAILURES_PER_MINUTE;
use crate::logging::ClientIp;

/// How long a window lasts.
const PERIOD: Duration = Duration::from_mins(1);

/// How long the hourly window of signing up lasts.
const HOUR: Duration = Duration::from_hours(1);

/// Each window remembers at most this many clients. A flood of invented addresses must not be
/// able to use up the memory. When the table is full, expired entries go first; counters of
/// clients that are still within their minute are never thrown away. If it is still full, see
/// [`Overflow`].
const MAX_CLIENTS: usize = 100_000;

/// A full table is swept for expired counters at most this often.
const SWEEP_EVERY: Duration = Duration::from_secs(1);

/// Requests of the whole API, per client network and minute, when nothing else is set.
pub const DEFAULT_REQUESTS_PER_MINUTE: u32 = 600;

/// All sign-in-family tries (right or wrong) per client network and minute, when nothing else is
/// set. This is a generous cap against floods; the failures are limited much more tightly
/// (`crate::failures`, [`DEFAULT_SIGN_IN_FAILURES_PER_MINUTE`]).
pub const DEFAULT_SIGN_IN_ATTEMPTS_PER_MINUTE: u32 = 60;

/// Tries to sign up per client network and minute. Signing up sends an e-mail to any address that
/// is typed, so it has a small limit of its own besides the cap of the sign-in family.
pub const SIGN_UP_REQUESTS_PER_MINUTE: u32 = 5;

/// Tries to sign up per client network and hour: well under the cap of the whole server
/// (`bb_core::sign_up_invitation::ADDRESSES_PER_HOUR_MAX`), so that one network cannot use it up
/// for everybody.
pub const SIGN_UP_REQUESTS_PER_HOUR: u32 = 20;

// Held in code: one network must not be able to fill a fifth of the cap on its own, so that it takes
// five networks to use it up and no fewer.
const _: () = assert!(
    (SIGN_UP_REQUESTS_PER_HOUR as u64) * 5 <= bb_core::sign_up_invitation::ADDRESSES_PER_HOUR_MAX
);

/// Requests that come with one access key, per minute, when nothing else is set. A script that
/// needs more asks for fewer, bigger answers (`limit`, `fields`) or sets the limit higher.
pub const DEFAULT_ACCESS_KEY_REQUESTS_PER_MINUTE: u32 = 300;

/// Searches per person and minute (SEC-17, architecture §7 *Search limits*): a search is the
/// heaviest read there is, so each person has a small allowance of their own beside the one of the
/// network. The sixty-first waits for the next minute.
pub const SEARCHES_PER_MINUTE: u32 = 60;

/// The limits, from the start-up settings (OPS-10; they move to the settings registry later).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LimitsConfig {
    /// Requests to `/api` per client network and minute.
    pub requests_per_minute: u32,
    /// All sign-in-family tries per client network and minute.
    pub sign_in_attempts_per_minute: u32,
    /// Failed tries per account, person and client network and window.
    pub sign_in_failures_per_minute: u32,
    /// Requests per minute that come with one access key.
    pub access_key_requests_per_minute: u32,
    /// Searches per person and minute, whether they come with a session or with a key.
    pub searches_per_minute: u32,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: DEFAULT_REQUESTS_PER_MINUTE,
            sign_in_attempts_per_minute: DEFAULT_SIGN_IN_ATTEMPTS_PER_MINUTE,
            sign_in_failures_per_minute: DEFAULT_SIGN_IN_FAILURES_PER_MINUTE,
            access_key_requests_per_minute: DEFAULT_ACCESS_KEY_REQUESTS_PER_MINUTE,
            searches_per_minute: SEARCHES_PER_MINUTE,
        }
    }
}

struct Entry {
    started: Instant,
    count: u32,
}

/// What a window does with a new key when its table is full of unexpired counters.
enum Overflow<K> {
    /// All such keys share one counter. Used for the general limit: a flood of invented clients
    /// ends up sharing one limit, while the clients already counted keep theirs.
    Share(K),
    /// The new key is refused (counts as over the limit). Used for sign-in: fail closed.
    Refuse,
}

/// Counts events per key in fixed windows.
struct Window<K> {
    limit: u32,
    /// How long a window lasts.
    period: Duration,
    overflow: Overflow<K>,
    table: Mutex<Table<K>>,
}

/// The counters of one window.
struct Table<K> {
    entries: HashMap<K, Entry>,
    /// When expired entries were last swept out. A full table is swept at most once per
    /// [`SWEEP_EVERY`], so a flood cannot make every request scan 100,000 entries.
    last_sweep: Option<Instant>,
    /// How many sweeps ran (tests count them).
    #[cfg(test)]
    sweeps: u32,
}

impl<K: Hash + Eq + Clone> Window<K> {
    fn new(limit: u32, overflow: Overflow<K>) -> Self {
        Self::lasting(PERIOD, limit, overflow)
    }

    fn lasting(period: Duration, limit: u32, overflow: Overflow<K>) -> Self {
        Self {
            limit,
            period,
            overflow,
            table: Mutex::new(Table {
                entries: HashMap::new(),
                last_sweep: None,
                #[cfg(test)]
                sweeps: 0,
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Table<K>> {
        self.table.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Counts one event. `Err` holds the seconds to wait (at least one) when the key is over
    /// the limit.
    fn hit(&self, key: K, now: Instant) -> Result<(), u64> {
        let mut table = self.lock();
        let mut key = key;
        if table.entries.len() >= MAX_CLIENTS && !table.entries.contains_key(&key) {
            create_room(&mut table, now, self.period);
            if table.entries.len() >= MAX_CLIENTS {
                match &self.overflow {
                    Overflow::Share(shared) => key = shared.clone(),
                    Overflow::Refuse => return Err(self.period.as_secs()),
                }
            }
        }
        let entry = table.entries.entry(key).or_insert(Entry {
            started: now,
            count: 0,
        });
        if now.duration_since(entry.started) >= self.period {
            *entry = Entry {
                started: now,
                count: 0,
            };
        }
        if entry.count >= self.limit {
            return Err(seconds_left(entry.started, now, self.period));
        }
        entry.count += 1;
        Ok(())
    }
}

/// The seconds a key that started its window at `started` still has to wait: at least one.
fn seconds_left(started: Instant, now: Instant, period: Duration) -> u64 {
    let wait = period.saturating_sub(now.duration_since(started));
    (wait.as_secs() + u64::from(wait.subsec_nanos() > 0)).max(1)
}

/// Throws out the counters whose window is over, when the table is full. A full table is swept
/// at most once per [`SWEEP_EVERY`]; counters that are still running are never thrown away.
fn create_room<K>(table: &mut Table<K>, now: Instant, period: Duration) {
    let due = table
        .last_sweep
        .is_none_or(|last| now.saturating_duration_since(last) >= SWEEP_EVERY);
    if due {
        table
            .entries
            .retain(|_, entry| now.duration_since(entry.started) < period);
        table.last_sweep = Some(now);
        #[cfg(test)]
        {
            table.sweeps += 1;
        }
    }
}

/// All limits of the server.
pub struct Limits {
    requests: Window<IpAddr>,
    sign_in_networks: Window<NetworkPrefix>,
    sign_up_networks: Window<NetworkPrefix>,
    sign_up_hours: Window<NetworkPrefix>,
    access_keys: Window<Digest>,
    searches: Window<UserId>,
}

impl Limits {
    /// The limits with the given numbers. A limit of `0` refuses everything.
    #[must_use]
    pub fn new(config: LimitsConfig) -> Arc<Self> {
        Arc::new(Self {
            requests: Window::new(config.requests_per_minute, Overflow::Share(OVERFLOW_CLIENT)),
            sign_in_networks: Window::new(config.sign_in_attempts_per_minute, Overflow::Refuse),
            sign_up_networks: Window::new(SIGN_UP_REQUESTS_PER_MINUTE, Overflow::Refuse),
            sign_up_hours: Window::lasting(HOUR, SIGN_UP_REQUESTS_PER_HOUR, Overflow::Refuse),
            // Keys that do not fit in a full table share one counter: a flood of invented keys
            // limits itself and not the keys that are counted already.
            access_keys: Window::new(
                config.access_key_requests_per_minute,
                Overflow::Share(SHARED_KEY),
            ),
            // A full table refuses a person it does not know yet: a search is not worth more
            // than the rest of the server.
            searches: Window::new(config.searches_per_minute, Overflow::Refuse),
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

    /// Counts one request that came with the access key `key` (the hash of the key, or
    /// [`MALFORMED_KEY`] for something that starts like a key and is none).
    ///
    /// # Errors
    /// `429` with `Retry-After` when the key is over its limit.
    pub fn check_access_key(&self, key: Digest) -> Result<(), ApiError> {
        self.check_access_key_at(key, Instant::now())
    }

    fn check_access_key_at(&self, key: Digest, now: Instant) -> Result<(), ApiError> {
        self.access_keys.hit(key, now).map_err(too_many)
    }

    /// Counts one search of `person` against their allowance of [`SEARCHES_PER_MINUTE`].
    ///
    /// # Errors
    /// `429` with `Retry-After` when the person is over it.
    pub fn check_search(&self, person: UserId) -> Result<(), ApiError> {
        self.check_search_at(person, Instant::now())
    }

    fn check_search_at(&self, person: UserId, now: Instant) -> Result<(), ApiError> {
        self.searches.hit(person, now).map_err(too_many)
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
            .hit(NetworkPrefix::from_address(client), now)
            .map_err(too_many)
    }
}

impl Limits {
    /// Counts one try to sign up of `client`.
    ///
    /// # Errors
    /// `429` with `Retry-After` when the client network is over its limit.
    pub fn check_sign_up(&self, client: IpAddr) -> Result<(), ApiError> {
        self.check_sign_up_at(client, Instant::now())
    }

    fn check_sign_up_at(&self, client: IpAddr, now: Instant) -> Result<(), ApiError> {
        let network = NetworkPrefix::from_address(client);
        self.sign_up_networks
            .hit(network.clone(), now)
            .map_err(too_many)?;
        self.sign_up_hours.hit(network, now).map_err(too_many)
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

/// The shared counter for clients that do not fit in a full table (an address no client has).
const OVERFLOW_CLIENT: IpAddr = IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED);

/// The counter that everything shares which starts like an access key and is none: it cannot
/// use up the limit of a key that exists, because that has its own counter.
pub(crate) const MALFORMED_KEY: Digest = Digest::new([0xFF; 32]);

/// The counter that keys share when the table is full of counters that are still running.
const SHARED_KEY: Digest = Digest::new([0x00; 32]);

/// A request without a known client address (never the case behind the real listener) is
/// counted with all others like it, so a wiring mistake limits instead of opening everything.
const UNKNOWN_CLIENT: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);

fn client_of(request: &Request) -> IpAddr {
    request
        .extensions()
        .get::<ClientIp>()
        .map_or(UNKNOWN_CLIENT, |ClientIp(address)| *address)
}

/// The middleware of the whole API: counts a request below `/api` against its client network.
/// Pages, files and health checks are not counted.
pub(crate) async fn limit_requests(
    State(limits): State<Arc<Limits>>,
    request: Request,
    next: Next,
) -> Response {
    if !crate::api::is_api(request.uri().path()) {
        return next.run(request).await;
    }
    if let Err(error) = limits.check_request(client_of(&request)) {
        return error.into_response();
    }
    next.run(request).await
}

/// The middleware of the whole API: counts a request that comes with an access key against the
/// key (`Authorization: Bearer bb_…`). A request without one passes untouched.
pub(crate) async fn limit_access_keys(
    State(limits): State<Arc<Limits>>,
    request: Request,
    next: Next,
) -> Response {
    if let Some(key) = crate::api::presented_key(request.headers())
        && crate::api::is_api(request.uri().path())
    {
        let counted =
            bb_auth::AccessKeySecret::parse(key).map_or(MALFORMED_KEY, |secret| secret.hash());
        if let Err(error) = limits.check_access_key(counted) {
            return error.into_response();
        }
    }
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

/// The middleware of the sign-up route: counts the try against the client network, with the small
/// limit for signing up.
pub async fn limit_sign_up(
    State(limits): State<Arc<Limits>>,
    request: Request,
    next: Next,
) -> Response {
    if let Err(error) = limits.check_sign_up(client_of(&request)) {
        return error.into_response();
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits(requests: u32, sign_in: u32) -> Arc<Limits> {
        Limits::new(LimitsConfig {
            requests_per_minute: requests,
            sign_in_attempts_per_minute: sign_in,
            ..LimitsConfig::default()
        })
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    fn wait_of(error: &ApiError) -> u64 {
        let response = error.clone().into_response();
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
    fn sec_17_the_sixty_first_search_in_a_minute_waits() {
        let limits = limits(1_000, 1_000);
        let start = Instant::now();
        let mut source = bb_core::id::SequentialIds::default();
        let (person, other) = (UserId::generate(&mut source), UserId::generate(&mut source));
        for _ in 0..SEARCHES_PER_MINUTE {
            limits.check_search_at(person, start).unwrap();
        }
        let error = limits
            .check_search_at(person, start + Duration::from_secs(20))
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::TooManyRequests);
        assert_eq!(wait_of(&error), 40);
        // Each person has an allowance of their own, and the minute ends.
        limits.check_search_at(other, start).unwrap();
        limits
            .check_search_at(person, start + Duration::from_secs(61))
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
    fn sec_14_signing_up_has_a_small_limit_of_its_own_per_network() {
        let limits = limits(1000, 1000);
        let (now, client) = (Instant::now(), ip("203.0.113.7"));
        for _ in 0..SIGN_UP_REQUESTS_PER_MINUTE {
            limits.check_sign_up_at(client, now).unwrap();
        }
        let error = limits.check_sign_up_at(client, now).unwrap_err();
        assert_eq!(error.code(), ErrorCode::TooManyRequests);
        // Another network, and the sign-in cap, are not touched by it.
        limits.check_sign_up_at(ip("198.51.100.1"), now).unwrap();
        limits.check_sign_in_at(client, now).unwrap();
    }

    #[test]
    fn sec_14_signing_up_has_an_hourly_limit_per_network_that_the_minutes_do_not_reach() {
        let limits = limits(1000, 1000);
        let (start, client) = (Instant::now(), ip("203.0.113.7"));
        // Spread over the hour, a network stays under the limit of each minute and is still
        // stopped by the limit of the hour.
        for number in 0..SIGN_UP_REQUESTS_PER_HOUR {
            let now = start + PERIOD * (number + 1);
            limits.check_sign_up_at(client, now).unwrap();
        }
        let now = start + PERIOD * (SIGN_UP_REQUESTS_PER_HOUR + 1);
        let error = limits.check_sign_up_at(client, now).unwrap_err();
        assert_eq!(error.code(), ErrorCode::TooManyRequests);
        // Another network is not touched, and an hour later the first one is let in again.
        limits.check_sign_up_at(ip("198.51.100.1"), now).unwrap();
        limits.check_sign_up_at(client, now + HOUR).unwrap();
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
    }

    #[test]
    fn sec_14_the_sign_in_cap_counts_per_network_in_the_size_the_failure_keys_use() {
        let limits = limits(1000, 2);
        let now = Instant::now();
        // IPv4: the `/24`. Two clients of one network share the cap, another network does not.
        limits.check_sign_in_at(ip("203.0.113.7"), now).unwrap();
        limits.check_sign_in_at(ip("203.0.113.200"), now).unwrap();
        assert!(limits.check_sign_in_at(ip("203.0.113.9"), now).is_err());
        limits.check_sign_in_at(ip("203.0.114.7"), now).unwrap();
        // IPv6: the `/48`, also for an IPv4 address written as IPv6.
        limits.check_sign_in_at(ip("2001:db8:1:2::1"), now).unwrap();
        limits
            .check_sign_in_at(ip("2001:db8:1:ffff::9"), now)
            .unwrap();
        assert!(limits.check_sign_in_at(ip("2001:db8:1:5::3"), now).is_err());
        limits.check_sign_in_at(ip("2001:db8:2::1"), now).unwrap();
        assert!(
            limits
                .check_sign_in_at(ip("::ffff:203.0.113.7"), now)
                .is_err()
        );
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

    fn fill(window: &Window<u32>, now: Instant) {
        for key in 0..u32::try_from(MAX_CLIENTS).unwrap() {
            window.hit(key, now).unwrap();
        }
    }

    #[test]
    fn sec_14_a_full_table_never_forgets_counters_that_are_still_running() {
        let window: Window<u32> = Window::new(1, Overflow::Share(u32::MAX));
        let now = Instant::now();
        fill(&window, now);
        // A client already counted keeps its counter, also while the table is full.
        assert!(window.hit(7, now).is_err());
        // New clients share one counter: the first passes, the next are over the limit.
        window.hit(u32::MAX - 1, now).unwrap();
        assert!(window.hit(u32::MAX - 2, now).is_err());
        assert!(window.table.lock().unwrap().entries.len() <= MAX_CLIENTS + 1);
        assert!(window.hit(7, now).is_err());
        // Expired counters make room again.
        let later = now + PERIOD + Duration::from_secs(1);
        window.hit(u32::MAX - 3, later).unwrap();
        window.hit(8, later).unwrap();
    }

    #[test]
    fn sec_14_a_full_table_is_swept_at_most_once_a_second_not_on_every_request() {
        let window: Window<u32> = Window::new(1, Overflow::Share(u32::MAX));
        let now = Instant::now();
        fill(&window, now);
        for new in 0..1000 {
            let _ = window.hit(
                u32::try_from(MAX_CLIENTS).unwrap() + new,
                now + Duration::from_millis(u64::from(new)),
            );
        }
        // 1000 requests over about one second: one sweep, maybe two at the second's edge.
        let sweeps = window.table.lock().unwrap().sweeps;
        assert!((1..=2).contains(&sweeps), "{sweeps}");
        // Much later the sweep runs again and frees the expired counters.
        let later = now + PERIOD + Duration::from_secs(5);
        window.hit(u32::MAX - 5, later).unwrap();
        assert!(window.table.lock().unwrap().entries.len() < 10);
    }

    #[test]
    fn sec_14_sign_in_refuses_new_keys_when_the_table_is_full() {
        let window: Window<u32> = Window::new(5, Overflow::Refuse);
        let now = Instant::now();
        fill(&window, now);
        let wait = window.hit(u32::MAX, now).unwrap_err();
        assert_eq!(wait, PERIOD.as_secs());
        // Known keys still work; after expiry a new key is accepted.
        window.hit(3, now).unwrap();
        window
            .hit(u32::MAX, now + PERIOD + Duration::from_secs(1))
            .unwrap();
    }
}
