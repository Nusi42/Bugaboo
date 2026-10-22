//! The adapter against a mail server that is a few lines of a test: what it receives is what a
//! mail program would be shown.

use bb_app::mailer::{MailFailure, MailMessage, MailServer, Mailer};
use bb_core::mail::MailSecurity;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use zeroize::Zeroizing;

use super::SmtpMailer;
/// What a test mail server saw of one e-mail.
struct Received {
    /// The commands before the text (`MAIL FROM`, `RCPT TO`).
    envelope: Vec<String>,
    /// The headers and the text.
    data: String,
}

/// Starts a mail server on a free port of this machine that takes one e-mail, answering `refuse`
/// to the first `RCPT TO` when it is given. Returns its port and what it receives.
async fn serve(refuse: Option<&'static str>) -> (u16, tokio::task::JoinHandle<Received>) {
    logged_so_far();
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
    let port = listener.local_addr().expect("an address").port();
    let task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("a connection");
        let (read, mut write) = stream.into_split();
        let mut lines = BufReader::new(read).lines();
        write.write_all(b"220 test ESMTP\r\n").await.expect("write");
        let mut envelope = Vec::new();
        let mut data = String::new();
        while let Some(line) = lines.next_line().await.expect("a line") {
            let command = line.to_ascii_uppercase();
            if command.starts_with("EHLO") {
                write.write_all(b"250 test\r\n").await.expect("write");
            } else if command.starts_with("MAIL FROM") {
                envelope.push(line);
                write.write_all(b"250 ok\r\n").await.expect("write");
            } else if command.starts_with("RCPT TO") {
                envelope.push(line);
                let answer = refuse.unwrap_or("250 ok");
                write
                    .write_all(format!("{answer}\r\n").as_bytes())
                    .await
                    .expect("write");
            } else if command == "DATA" {
                write.write_all(b"354 go on\r\n").await.expect("write");
                while let Some(text) = lines.next_line().await.expect("a line") {
                    if text == "." {
                        break;
                    }
                    data.push_str(&text);
                    data.push('\n');
                }
                write.write_all(b"250 queued\r\n").await.expect("write");
            } else if command == "QUIT" {
                write.write_all(b"221 bye\r\n").await.expect("write");
                break;
            } else {
                write.write_all(b"250 ok\r\n").await.expect("write");
            }
        }
        Received { envelope, data }
    });
    (port, task)
}

fn server(port: u16) -> MailServer {
    MailServer {
        host: "127.0.0.1".to_owned(),
        port,
        security: MailSecurity::Off,
        username: String::new(),
        password: Zeroizing::new(String::new()),
        from: "Bugaboo <bugaboo@example.test>".to_owned(),
    }
}

fn message(subject: &str, text: &str) -> MailMessage {
    MailMessage {
        to: "ana@example.test".to_owned(),
        subject: subject.to_owned(),
        text: text.to_owned(),
    }
}

#[tokio::test]
async fn ntf_02_an_e_mail_is_plain_text_and_what_it_says_is_not_run_or_loaded() {
    let (port, received) = serve(None).await;
    let text = "<script>alert(1)</script>
![a picture](https://elsewhere.example/p.png)
<img src=\"https://elsewhere.example/p.png\">";
    SmtpMailer
        .send(&server(port), &message("A title", text))
        .await
        .expect("the e-mail goes");
    let received = received.await.expect("the server finished");
    assert!(received.envelope[0].contains("bugaboo@example.test"));
    assert!(received.envelope[1].contains("ana@example.test"));
    assert!(received.data.contains("Content-Type: text/plain"));
    assert!(!received.data.to_ascii_lowercase().contains("text/html"));
    assert!(!received.data.to_ascii_lowercase().contains("multipart"));
}

#[tokio::test]
async fn ntf_02_a_subject_made_of_a_title_cannot_add_a_header() {
    let (port, received) = serve(None).await;
    SmtpMailer
        .send(
            &server(port),
            &message(
                "Title
Bcc: spy@example.test

Injected body",
                "Text",
            ),
        )
        .await
        .expect("the e-mail goes");
    let received = received.await.expect("the server finished");
    assert!(!received.data.lines().any(|line| line.starts_with("Bcc:")));
    assert!(received.envelope.iter().all(|line| !line.contains("spy@")));
}

#[tokio::test]
async fn a_mail_server_that_refuses_for_good_is_a_rejection() {
    let (port, received) = serve(Some("550 5.1.1 no such mailbox")).await;
    let outcome = SmtpMailer
        .send(&server(port), &message("Subject", "Text"))
        .await;
    assert_eq!(outcome, Err(MailFailure::Rejected));
    received.abort();
}

#[tokio::test]
async fn a_mail_server_that_says_try_later_is_unavailable() {
    let (port, received) = serve(Some("451 4.3.0 try again later")).await;
    let outcome = SmtpMailer
        .send(&server(port), &message("Subject", "Text"))
        .await;
    assert_eq!(outcome, Err(MailFailure::Unavailable));
    received.abort();
}

#[tokio::test]
async fn a_mail_server_that_is_not_there_is_unavailable() {
    // A port that nobody listens on: the one the test server had, after it is gone.
    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
        listener.local_addr().expect("an address").port()
    };
    let outcome = SmtpMailer
        .send(&server(port), &message("Subject", "Text"))
        .await;
    assert_eq!(outcome, Err(MailFailure::Unavailable));
}

#[tokio::test]
async fn an_address_that_is_none_is_a_rejection_before_anything_is_sent() {
    let mut bad = message("Subject", "Text");
    bad.to = "not an address".to_owned();
    assert_eq!(
        SmtpMailer.send(&server(1), &bad).await,
        Err(MailFailure::Rejected)
    );
}

#[tokio::test]
async fn sec_m4_a_recipient_in_mailbox_form_is_a_rejection_never_a_second_address() {
    for bad in [
        "x<a@b.test>",
        "x <a@b.test>",
        "\"x\" <a@b.test>",
        "a@b.test, c@d.test",
    ] {
        let mut mail = message("Subject", "Text");
        mail.to = bad.to_owned();
        assert_eq!(
            SmtpMailer.send(&server(1), &mail).await,
            Err(MailFailure::Rejected),
            "{bad}"
        );
    }
}

#[tokio::test]
async fn sec_m4_a_plus_tag_goes_to_the_whole_address() {
    let (port, received) = serve(None).await;
    let mut mail = message("Subject", "Text");
    mail.to = "a+1@example.test".to_owned();
    SmtpMailer
        .send(&server(port), &mail)
        .await
        .expect("the e-mail goes");
    let received = received.await.expect("the server finished");
    assert!(received.envelope[1].contains("<a+1@example.test>"));
}

/// Collects what the adapter logs, so a test can read it.
#[derive(Clone, Default)]
struct Captured(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Captured {
    type Writer = Self;

    fn make_writer(&'a self) -> Self {
        self.clone()
    }
}

/// Everything the tests of this module log. The subscriber is global: a log statement remembers
/// that nobody listens when it first runs without one, so a subscriber of one test would miss it.
fn logged_so_far() -> &'static Captured {
    static LOG: std::sync::OnceLock<Captured> = std::sync::OnceLock::new();
    LOG.get_or_init(|| {
        let captured = Captured::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(captured.clone())
            .with_ansi(false)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
        captured
    })
}

#[tokio::test]
async fn data_logs_no_e_mail_address_in_the_log() {
    let captured = logged_so_far();
    // The answer of the server names the address, as real servers do.
    let (port, received) = serve(Some("550 5.1.1 <ana@example.test> no such mailbox")).await;
    let outcome = SmtpMailer
        .send(&server(port), &message("Subject", "Text"))
        .await;
    assert_eq!(outcome, Err(MailFailure::Rejected));
    received.abort();
    let logged = String::from_utf8_lossy(
        &captured
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    )
    .into_owned();
    assert!(logged.contains("did not take an e-mail"), "{logged}");
    assert!(!logged.contains("ana@"), "{logged}");
    assert!(!logged.contains("example.test"), "{logged}");
}
