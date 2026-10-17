//! Security headers on every response (SEC-11, architecture §7 "Browser protections").
//!
//! The values are defined once, here, and the integration tests in `tests/` read them from here.
//! SEC-HEADERS holds this table against the requirement (SEC-11): every required header is set, the
//! policy has no source that lets injected or foreign code run, and HSTS never reaches
//! subdomains.

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::{Request, State};
use axum::http::{HeaderName, HeaderValue, header};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::Response;
use bb_app::auth::RequestInfo;
use bb_app::settings::DocumentImagesQuery;
use bb_auth::Device;
use tokio::sync::Mutex;
use tower_http::set_header::SetResponseHeaderLayer;

use crate::api::AuthHandle;

/// Headers every response carries.
pub const SECURITY_HEADERS: [(&str, &str); 6] = [
    (
        "content-security-policy",
        concat!(
            "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; \
             img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; \
             base-uri 'none'; form-action 'self'; require-trusted-types-for 'script'; \
             trusted-types ",
            bb_api_words::trusted_types_policy!()
        ),
    ),
    ("x-content-type-options", "nosniff"),
    ("referrer-policy", "same-origin"),
    ("cross-origin-opener-policy", "same-origin"),
    ("cross-origin-resource-policy", "same-origin"),
    (
        "permissions-policy",
        "accelerometer=(), camera=(), display-capture=(), geolocation=(), gyroscope=(), \
         magnetometer=(), microphone=(), payment=(), usb=(), browsing-topics=()",
    ),
];

/// The policy of a download (ADR-0032 §3): the content may load nothing, no page may frame it, and
/// even a page it would make of itself runs in a sandbox with no right at all. A file is only ever
/// saved; this holds if a browser shows it anyway. It replaces the policy of the table on those
/// answers, and SEC-HEADERS holds it against the requirement that a policy allows nothing, and
/// that no other code sets the policy header.
pub const DOWNLOAD_POLICY: &str = "default-src 'none'; frame-ancestors 'none'; sandbox";

/// The name of the policy header: the first entry of the table (a test holds it to that), so the
/// name is written once.
const POLICY: &str = SECURITY_HEADERS[0].0;
/// The directive of the table that the hosts of `content.image-hosts` are added to (M3-F10).
const IMG_SRC: &str = "img-src 'self' data:";
/// How long a page load may use the image hosts it read before: a change of the setting shows in
/// documents loaded after this long.
const HOSTS_KEPT: Duration = Duration::from_secs(10);

/// The policy with the image hosts of the organization added to `img-src`. Images always come over
/// HTTPS, so the sources say so.
#[must_use]
pub(crate) fn with_image_hosts(policy: &str, hosts: &[String]) -> String {
    let sources: Vec<String> = hosts.iter().map(|host| format!("https://{host}")).collect();
    if sources.is_empty() {
        return policy.to_owned();
    }
    policy.replacen(IMG_SRC, &format!("{IMG_SRC} {}", sources.join(" ")), 1)
}

/// The image hosts of the organization, read for the documents that are sent, and kept for a few
/// seconds so a page load costs no read of its own.
struct ImageHostsCache {
    auth: AuthHandle,
    kept: Mutex<Option<(Instant, Vec<String>)>>,
    /// Held while one request reads, so a burst of page loads costs one read, and the lock of
    /// `kept` is never held across the read of the database.
    refreshing: Mutex<()>,
}

impl ImageHostsCache {
    /// The hosts that were read lately, if there are any.
    async fn fresh(&self) -> Option<Vec<String>> {
        let kept = self.kept.lock().await;
        kept.as_ref()
            .filter(|(read_at, _)| read_at.elapsed() < HOSTS_KEPT)
            .map(|(_, hosts)| hosts.clone())
    }

    async fn hosts(&self) -> Vec<String> {
        if let Some(hosts) = self.fresh().await {
            return hosts;
        }
        let _turn = self.refreshing.lock().await;
        // Another request may have read while this one waited for its turn.
        if let Some(hosts) = self.fresh().await {
            return hosts;
        }
        let request = RequestInfo {
            network: None,
            device: Device::default(),
            background: false,
        };
        // Without a database, or when the read fails, the base policy applies: no image from
        // outside. That answer is not kept: the next page load reads again, so a read that failed
        // once does not shut the images out for the time a good answer is kept.
        let Ok(api) = self.auth.api() else {
            return Vec::new();
        };
        let hosts = match api.document_image_hosts(request, DocumentImagesQuery).await {
            Ok(found) => found.hosts,
            Err(error) => {
                tracing::warn!(cause = %error, "the image hosts could not be read; no image from outside for this page");
                return Vec::new();
            }
        };
        *self.kept.lock().await = Some((Instant::now(), hosts.clone()));
        hosts
    }
}

/// The policy of a document (a page of the app) carries the image hosts of the organization; the
/// answers of the API keep the table's policy.
async fn policy_of_documents(
    State(cache): State<Arc<ImageHostsCache>>,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let is_document = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/html"));
    if !is_document {
        return response;
    }
    let hosts = cache.hosts().await;
    let current = response
        .headers()
        .get(POLICY)
        .and_then(|value| value.to_str().ok())
        .map(|policy| with_image_hosts(policy, &hosts));
    if let Some(value) = current.and_then(|policy| HeaderValue::from_str(&policy).ok()) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(POLICY), value);
    }
    response
}

/// Sent only when browsers reach us over HTTPS (here, or at a proxy we trust). Never with
/// `includeSubDomains`: Bugaboo does not own the other names of its host.
pub const HSTS_NAME: &str = "strict-transport-security";

/// With a certificate Bugaboo made itself: one day. A person who trusts that certificate on
/// `localhost` must not pin every other local tool to HTTPS for a long time.
pub const HSTS_MAX_AGE_SELF_SIGNED: u64 = 86_400;

/// With a certificate the owner provided, or HTTPS at a trusted proxy: one year.
pub const HSTS_MAX_AGE_TRUSTED: u64 = 31_536_000;

/// The value of the HSTS header for a lifetime in seconds.
#[must_use]
pub fn hsts_value(max_age: u64) -> String {
    format!("max-age={max_age}")
}

/// Wraps the router so every response, including errors and fallbacks, carries the headers.
/// `hsts_max_age` is the HSTS lifetime in seconds, or `None` when the connection is not HTTPS.
pub(crate) fn apply(router: Router, hsts_max_age: Option<u64>, auth: AuthHandle) -> Router {
    let hsts = hsts_max_age.map(|seconds| (HSTS_NAME, hsts_value(seconds)));
    let fixed = SECURITY_HEADERS
        .iter()
        .map(|&(name, value)| (name, value.to_owned()));
    let cache = Arc::new(ImageHostsCache {
        auth,
        kept: Mutex::new(None),
        refreshing: Mutex::new(()),
    });
    let router = fixed.chain(hsts).fold(router, |router, (name, value)| {
        match HeaderValue::from_str(&value) {
            // The policy of the table is the one every answer gets, unless the answer brought a
            // stricter one of its own: a download (`DOWNLOAD_POLICY`).
            Ok(value) if name == POLICY => router.layer(SetResponseHeaderLayer::if_not_present(
                HeaderName::from_static(name),
                value,
            )),
            Ok(value) => router.layer(SetResponseHeaderLayer::overriding(
                HeaderName::from_static(name),
                value,
            )),
            // Every value is ASCII text written above; an invalid one would be a coding
            // mistake that the header tests catch.
            Err(_) => router,
        }
    });
    // Outside the layers above: they set the table's policy on every answer, this one then adds
    // the image hosts to the policy of a document.
    router.layer(from_fn_with_state(cache, policy_of_documents))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> &'static str {
        SECURITY_HEADERS
            .iter()
            .find(|(name, _)| name.starts_with("content-security"))
            .map_or("", |(_, value)| *value)
    }

    #[test]
    fn m3_f10_the_first_entry_of_the_table_is_the_policy() {
        assert!(POLICY.starts_with("content-security"));
        assert!(SECURITY_HEADERS[0].1.contains(IMG_SRC));
    }

    #[test]
    fn m3_f10_image_hosts_join_img_src_and_nothing_else() {
        let hosts = [
            "images.example.com".to_owned(),
            "cdn.example.org:8443".to_owned(),
        ];
        let widened = with_image_hosts(policy(), &hosts);
        assert!(widened.contains(
            "img-src 'self' data: https://images.example.com https://cdn.example.org:8443;"
        ));
        assert_eq!(
            widened.replace(
                " https://images.example.com https://cdn.example.org:8443",
                ""
            ),
            policy()
        );
        assert_eq!(with_image_hosts(policy(), &[]), policy());
    }

    #[test]
    fn sec_13_the_csp_allows_exactly_the_one_trusted_types_policy() {
        let directives: Vec<&str> = policy().split(';').map(str::trim).collect();
        let allowed: Vec<&&str> = directives
            .iter()
            .filter(|directive| directive.starts_with("trusted-types"))
            .collect();
        let expected = format!("trusted-types {}", bb_api_words::trusted_types::POLICY);
        // One directive, with the one name and nothing after it; it is the last of the policy.
        assert_eq!(allowed, [&expected.as_str()]);
        assert!(policy().ends_with(&expected));
        assert!(!policy().contains("allow-duplicates"));
        assert!(directives.contains(&"require-trusted-types-for 'script'"));
    }
}
