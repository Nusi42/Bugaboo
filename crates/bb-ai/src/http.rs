//! The one outgoing HTTP call of the server (ARCH-HTTP-EDGE): a POST with a JSON body to the
//! provider, with a time limit, a limit on the size of the answer, no redirect and HTTPS only
//! unless the address is a model on the organization's own network.
//!
//! `ureq` blocks, so the call runs on a thread of its own, never on a worker of the async runtime.
//! A failure is one class: nothing the provider sent back, and not the address, reaches a log or
//! the caller (DATA-LOGS).

use std::time::Duration;

use bb_core::ai::{ANSWER_BYTES_MAX, CALL_TIMEOUT_SECONDS, CONNECT_TIMEOUT_SECONDS};
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
}

/// Nothing was answered that can be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NoAnswer;

/// Makes the call on a thread of its own and gives back the body of a successful answer.
pub(crate) async fn post(call: Post) -> Result<String, NoAnswer> {
    tokio::task::spawn_blocking(move || post_blocking(&call))
        .await
        .map_err(|_| NoAnswer)?
}

fn post_blocking(call: &Post) -> Result<String, NoAnswer> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(CALL_TIMEOUT_SECONDS)))
        .timeout_connect(Some(Duration::from_secs(CONNECT_TIMEOUT_SECONDS)))
        // A redirect could lead to an address the checks never saw, with the key in the header.
        .max_redirects(0)
        .https_only(call.encrypted)
        .http_status_as_error(false)
        .build()
        .into();
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
    use std::net::TcpListener;

    use super::*;

    /// A server on this computer that answers one request with `answer` and tells what it got.
    fn serve_once(answer: &'static str) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let url = format!(
            "http://{}/v1/chat/completions",
            listener.local_addr().unwrap()
        );
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
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
                        break;
                    }
                }
            }
            stream.write_all(answer.as_bytes()).unwrap();
            String::from_utf8_lossy(&seen).into_owned()
        });
        (url, handle)
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
        }
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
