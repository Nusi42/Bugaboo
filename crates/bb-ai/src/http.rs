//! The one outgoing HTTP call of the server (ARCH-HTTP-EDGE): a POST with a JSON body to the
//! provider, with a time limit, a limit on the size of the answer, no redirect and HTTPS only
//! unless the address is a model on the organization's own network.
//!
//! The name of the provider is looked up once and judged, and the call goes to those addresses and
//! no others. Behind an outgoing proxy (`HTTPS_PROXY`, `ALL_PROXY`, … unless `NO_PROXY` names the
//! host) the proxy connects to the provider and looks its name up itself, so the judged addresses
//! cannot be held at the connection: the name is still judged, and a model on the organization's
//! own network is never called through a proxy, since the proxy, not the judged address, decides
//! where its data goes.
//!
//! `ureq` blocks, so the call runs on a thread of its own, never on a worker of the async runtime.
//! A failure is one class: nothing the provider sent back, and not the address, reaches a log or
//! the caller (DATA-LOGS).

use std::io;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs as _};
use std::time::Duration;

use bb_core::ai::{
    ANSWER_BYTES_MAX, CALL_TIMEOUT_SECONDS, CONNECT_TIMEOUT_SECONDS, judge_resolved,
};
use ureq::Proxy;
use ureq::http::Uri;
use ureq::unversioned::resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::{DefaultConnector, NextTimeout};
use zeroize::Zeroizing;

/// One call to make.
pub(crate) struct Post {
    /// The whole address.
    pub(crate) url: String,
    /// The headers besides the type of the content.
    pub(crate) headers: Vec<(&'static str, Zeroizing<String>)>,
    /// The JSON body.
    pub(crate) body: String,
    /// Whether the connection must be encrypted.
    pub(crate) encrypted: bool,
    /// Whether the model was judged to be on the organization's own network (`judge_resolved`).
    pub(crate) local: bool,
}

/// Looks a name up: the addresses of `host` at `port`. Given as a function, so the tests judge
/// addresses without a network.
pub(crate) type Lookup = fn(&str, u16) -> io::Result<Vec<IpAddr>>;

/// The resolver of the system.
fn system_lookup(host: &str, port: u16) -> io::Result<Vec<IpAddr>> {
    Ok((host, port)
        .to_socket_addrs()?
        .map(|address| address.ip())
        .collect())
}

/// The resolver of `ureq` for one call: the name of the provider answers with the addresses that
/// were judged, and no others; any other name (the one of a proxy) is looked up as usual. The name
/// stays in the address, so HTTPS still checks the certificate against the name.
#[derive(Debug)]
struct Judged {
    /// The name of the provider, as [`host_of`] writes it.
    host: String,
    /// What the name resolved to, with the port.
    addresses: Vec<SocketAddr>,
}

impl Resolver for Judged {
    fn resolve(
        &self,
        uri: &Uri,
        config: &ureq::config::Config,
        timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, ureq::Error> {
        if host_of(uri).is_none_or(|host| !host.eq_ignore_ascii_case(&self.host)) {
            return DefaultResolver::default().resolve(uri, config, timeout);
        }
        let mut addresses = self.empty();
        // Past what the array holds the rest is left out; the first ones are enough to connect.
        for address in &self.addresses {
            if addresses.try_push(*address).is_err() {
                break;
            }
        }
        Ok(addresses)
    }
}

/// The host of `uri` without the brackets of an IPv6 address.
fn host_of(uri: &Uri) -> Option<&str> {
    uri.host()
        .map(|host| host.trim_start_matches('[').trim_end_matches(']'))
}

/// The name of `uri` resolved once and judged (`judge_resolved`): the resolver that connects to
/// those addresses only.
fn judged(uri: &Uri, local: bool, lookup: Lookup) -> Result<Judged, NoAnswer> {
    let host = host_of(uri).ok_or(NoAnswer)?;
    let https = uri.scheme_str() == Some("https");
    let port = uri.port_u16().unwrap_or(if https { 443 } else { 80 });
    let resolved = lookup(host, port).map_err(|_| {
        tracing::warn!("the address of the AI provider could not be looked up");
        NoAnswer
    })?;
    judge_resolved(&resolved, local).map_err(|_| {
        tracing::warn!("the address of the AI provider is one the server never calls");
        NoAnswer
    })?;
    Ok(Judged {
        host: host.to_owned(),
        addresses: resolved
            .into_iter()
            .map(|address| SocketAddr::new(address, port))
            .collect(),
    })
}

/// Nothing was answered that can be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NoAnswer;

/// Makes the call on a thread of its own and gives back the body of a successful answer. The
/// outgoing proxy is the one the environment of the server names.
pub(crate) async fn post(call: Post) -> Result<String, NoAnswer> {
    post_through(call, system_lookup, Proxy::try_from_env()).await
}

/// [`post`], with the name looked up by `lookup` and through `proxy`.
async fn post_through(
    call: Post,
    lookup: Lookup,
    proxy: Option<Proxy>,
) -> Result<String, NoAnswer> {
    tokio::task::spawn_blocking(move || post_blocking(&call, lookup, proxy))
        .await
        .map_err(|_| NoAnswer)?
}

fn post_blocking(call: &Post, lookup: Lookup, proxy: Option<Proxy>) -> Result<String, NoAnswer> {
    let uri: Uri = call.url.parse().map_err(|_| NoAnswer)?;
    let proxy = proxy.filter(|proxy| !proxy.is_no_proxy(&uri));
    if proxy.is_some() && call.local {
        tracing::warn!(
            "the AI model is on the organization's own network, which is never called through the              outgoing proxy: add its name to NO_PROXY"
        );
        return Err(NoAnswer);
    }
    let judged = judged(&uri, call.local, lookup)?;
    let config = ureq::Agent::config_builder()
        .proxy(proxy)
        .timeout_global(Some(Duration::from_secs(CALL_TIMEOUT_SECONDS)))
        .timeout_connect(Some(Duration::from_secs(CONNECT_TIMEOUT_SECONDS)))
        // A redirect could lead to an address the checks never saw, with the key in the header.
        .max_redirects(0)
        .https_only(call.encrypted)
        .http_status_as_error(false)
        .build();
    let agent = ureq::Agent::with_parts(config, DefaultConnector::new(), judged);
    let mut request = agent
        .post(&call.url)
        .header("content-type", "application/json");
    for (name, value) in &call.headers {
        request = request.header(*name, value.as_str());
    }
    let mut response = request.send(call.body.as_str()).map_err(|error| {
        tracing::warn!(
            timed_out = matches!(error, ureq::Error::Timeout(_)),
            "the AI provider could not be reached"
        );
        NoAnswer
    })?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        tracing::warn!(status, "the AI provider refused the call");
        return Err(NoAnswer);
    }
    let limit = u64::try_from(ANSWER_BYTES_MAX).unwrap_or(u64::MAX);
    response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_string()
        .map_err(|_| {
            tracing::warn!("the answer of the AI provider could not be read");
            NoAnswer
        })
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    use super::*;

    /// Reads one request from `stream`: its head and as much body as it says it has.
    fn read_request(stream: &mut TcpStream) -> String {
        let mut seen = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let read = stream.read(&mut buffer).unwrap();
            seen.extend_from_slice(&buffer[..read]);
            let text = String::from_utf8_lossy(&seen).into_owned();
            if let Some((head, body)) = text.split_once("\r\n\r\n") {
                let wanted = head
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|length| length.trim().parse::<usize>().ok())
                    })
                    .unwrap_or_default();
                if body.len() >= wanted {
                    return text;
                }
            }
        }
    }

    /// A server on this computer that answers one request with `answer` and tells what it got.
    fn serve_once(answer: &'static str) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let url = format!(
            "http://{}/v1/chat/completions",
            listener.local_addr().unwrap()
        );
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let seen = read_request(&mut stream);
            stream.write_all(answer.as_bytes()).unwrap();
            seen
        });
        (url, handle)
    }

    /// An outgoing proxy on this computer that lets one tunnel through (`CONNECT`), answers the
    /// request in it with `answer`, and tells what it got: the `CONNECT`, then the request.
    fn proxy_once(answer: &'static str) -> (Proxy, std::thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let proxy = Proxy::new(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let connect = read_request(&mut stream);
            stream
                .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
                .unwrap();
            let request = read_request(&mut stream);
            stream.write_all(answer.as_bytes()).unwrap();
            vec![connect, request]
        });
        (proxy, handle)
    }

    /// A proxy that is never called: it only listens, so a test can see that nobody came.
    fn idle_proxy() -> (Proxy, TcpListener) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let proxy = Proxy::new(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        (proxy, listener)
    }

    fn nobody_came(listener: &TcpListener) -> bool {
        listener.set_nonblocking(true).unwrap();
        listener.accept().is_err()
    }

    fn call(url: String, encrypted: bool) -> Post {
        Post {
            url,
            headers: vec![(
                "authorization",
                Zeroizing::new("Bearer test-key".to_owned()),
            )],
            body: r#"{"model":"m"}"#.to_owned(),
            encrypted,
            local: false,
        }
    }

    /// A resolver that knows every name as this computer.
    const HERE: Lookup = |_, _| Ok(vec![IpAddr::from([127, 0, 0, 1])]);

    /// A resolver that answers with the metadata service of a cloud.
    const METADATA: Lookup = |_, _| Ok(vec![IpAddr::from([169, 254, 169, 254])]);

    /// A resolver that knows every name as an address on the internet nobody answers at.
    const OUT_THERE: Lookup = |_, _| Ok(vec![IpAddr::from([203, 0, 113, 7])]);

    /// A JSON answer of `{"ok":1}`.
    const OK: &str = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 8\r\nconnection: close\r\n\r\n{\"ok\":1}";

    #[tokio::test]
    async fn ai_01_a_call_behind_an_outgoing_proxy_goes_through_the_proxy() {
        let (proxy, seen) = proxy_once(OK);
        // The name of the proxy is looked up as usual, not answered with the provider's address.
        let body = post_through(
            call(
                "http://models.example.test/v1/chat/completions".to_owned(),
                false,
            ),
            OUT_THERE,
            Some(proxy),
        )
        .await
        .unwrap();
        assert_eq!(body, r#"{"ok":1}"#);
        let seen = seen.join().unwrap();
        assert!(
            seen[0].starts_with("CONNECT models.example.test:80 "),
            "{seen:?}"
        );
        assert!(
            seen[1].starts_with("POST /v1/chat/completions "),
            "{seen:?}"
        );
    }

    #[tokio::test]
    async fn ai_01_behind_a_proxy_the_name_is_still_judged() {
        let (proxy, listener) = idle_proxy();
        let outside = call("http://models.example.test/v1".to_owned(), false);
        assert_eq!(
            post_through(outside, METADATA, Some(proxy)).await,
            Err(NoAnswer)
        );
        assert!(nobody_came(&listener));
    }

    #[tokio::test]
    async fn ai_01_a_model_on_the_own_network_is_never_called_through_a_proxy() {
        let (proxy, listener) = idle_proxy();
        let local = Post {
            local: true,
            ..call("http://gpu-box:8000/v1/chat/completions".to_owned(), false)
        };
        assert_eq!(post_through(local, HERE, Some(proxy)).await, Err(NoAnswer));
        assert!(nobody_came(&listener));
    }

    #[tokio::test]
    async fn ai_01_a_model_that_no_proxy_names_is_called_directly() {
        let (url, seen) = serve_once(OK);
        let proxy = Proxy::builder(ureq::ProxyProtocol::Http)
            .host("127.0.0.1")
            .port(1)
            .no_proxy("gpu-box")
            .build()
            .unwrap();
        let local = Post {
            local: true,
            ..call(url.replace("127.0.0.1", "gpu-box"), false)
        };
        assert_eq!(
            post_through(local, HERE, Some(proxy)).await.unwrap(),
            r#"{"ok":1}"#
        );
        drop(seen);
    }

    #[tokio::test]
    async fn ai_01_the_call_goes_to_the_address_that_was_judged() {
        let (url, seen) = serve_once(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 8\r\nconnection: close\r\n\r\n{\"ok\":1}",
        );
        // A name nobody else knows reaches the server here, because the judged address is used.
        let named = url.replace("127.0.0.1", "gpu-box");
        let body = post_through(
            Post {
                local: true,
                ..call(named, false)
            },
            HERE,
            None,
        )
        .await
        .unwrap();
        assert_eq!(body, r#"{"ok":1}"#);
        assert!(
            seen.join()
                .unwrap()
                .to_ascii_lowercase()
                .contains("host: gpu-box")
        );
    }

    #[tokio::test]
    async fn ai_01_a_name_that_resolves_to_a_metadata_service_is_never_called() {
        let (url, seen) = serve_once("HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
        let named = url.replace("127.0.0.1", "models.example.test");
        assert_eq!(
            post_through(call(named, false), METADATA, None).await,
            Err(NoAnswer)
        );
        drop(seen);
    }

    #[tokio::test]
    async fn ai_01_a_model_on_this_computer_answers_over_plain_http() {
        let (url, seen) = serve_once(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 8\r\nconnection: close\r\n\r\n{\"ok\":1}",
        );
        let body = post(call(url, false)).await.unwrap();
        assert_eq!(body, r#"{"ok":1}"#);
        let request = seen.join().unwrap().to_ascii_lowercase();
        assert!(request.contains("authorization: bearer test-key"));
        assert!(request.contains("content-type: application/json"));
    }

    #[tokio::test]
    async fn ai_01_an_encrypted_call_never_goes_out_in_plain_http() {
        let (url, seen) = serve_once("HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
        assert_eq!(post(call(url, true)).await, Err(NoAnswer));
        drop(seen);
    }

    #[tokio::test]
    async fn ai_01_an_error_status_and_a_redirect_are_no_answer() {
        for answer in [
            "HTTP/1.1 500 Oops\r\ncontent-length: 4\r\nconnection: close\r\n\r\nboom",
            "HTTP/1.1 302 Found\r\nlocation: http://127.0.0.1:1/\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        ] {
            let (url, seen) = serve_once(answer);
            assert_eq!(post(call(url, false)).await, Err(NoAnswer), "{answer}");
            drop(seen);
        }
    }

    #[tokio::test]
    async fn ai_01_an_answer_over_the_limit_is_no_answer() {
        let big = "x".repeat(ANSWER_BYTES_MAX + 10);
        let answer: &'static str = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{big}",
                big.len()
            )
            .into_boxed_str(),
        );
        let (url, seen) = serve_once(answer);
        assert_eq!(post(call(url, false)).await, Err(NoAnswer));
        drop(seen);
    }
}
