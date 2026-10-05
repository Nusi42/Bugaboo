//! `bugaboo serve`: opens the data folder and the database, listens and serves until it is told
//! to stop (SEC-10, SEC-11, OPS-01).

use std::future::Future;
use std::io::IsTerminal as _;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use anyhow::Context as _;
use bb_api::tls::{ServedCertificate, TlsError, TlsSource, certificate_path, serve_certificate};
use bb_api::{
    HSTS_MAX_AGE_SELF_SIGNED, HSTS_MAX_AGE_TRUSTED, Incoming, ListenerOptions, Options, WebSource,
};
use bb_core::message::{self, Message};
use tokio::net::TcpListener;

use super::{ServeArgs, load_settings, renewal};
use crate::config::{Settings, TlsMode};
use crate::data_folder::{DataFolder, Existing, write_private};
use crate::wiring::Serving;
use crate::{log, output, wiring};

/// `bugaboo serve`: runs until Ctrl+C or the stop signal of the system.
pub(super) async fn serve(args: ServeArgs) -> anyhow::Result<()> {
    let settings = load_settings(args.config.as_deref(), &args.settings(), true)?;
    log::init(settings.log_level);
    let source = args.web.map_or(WebSource::Embedded, WebSource::Folder);
    // Installed before listening, so a failure is reported instead of stopping silently.
    let shutdown = shutdown_signal()?;
    serve_settings(settings, source, std::io::stdout().is_terminal(), shutdown).await
}

/// Runs the server with these settings until `shutdown` completes. A service passes the stop
/// request of the service manager; `bugaboo serve` passes the signals of the console. `console`
/// says whether somebody reads the output, which decides how the setup code is shown.
pub(super) async fn serve_settings(
    settings: Settings,
    source: WebSource,
    console: bool,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    let data = DataFolder::prepare(&settings.data_dir)?;
    let (listener_options, certificate) = listener_options(&settings, &data)?;
    let hsts_max_age = hsts_max_age(&settings);
    // The database, the keys and the sign-in services come first: when one of them fails the
    // server stops here, before it listens (M1-F19, M1-F24).
    let wired = wiring::wire(
        &settings,
        &data,
        Serving {
            certificate: certificate.clone(),
        },
    )
    .await?;
    // Before the port opens: whoever connects can already find the setup code.
    let setup_notice = prepare_setup_code(&data, wired.setup_code.as_deref(), console);
    let address = SocketAddr::new(settings.address, settings.port);
    let tcp = TcpListener::bind(address)
        .await
        .with_context(|| message::server_port_busy(settings.port).render())?;
    let incoming = Incoming::start(tcp, listener_options);

    if !source.has_index().await {
        output::warning(&Message::from(message::SERVER_WEB_APP_MISSING).render());
    }
    if settings.tls == TlsMode::SelfSigned {
        let path = certificate_path(&data.tls());
        output::line(&message::server_self_signed(path.display()).render());
    }
    output::line(&database_line(settings.database_url.as_deref()));
    let scheme = if settings.tls == TlsMode::Off {
        "http"
    } else {
        "https"
    };
    output::line(&message::server_listening(format!("{scheme}://{address}")).render());
    announce_setup_code(&setup_notice);
    tracing::info!(https = hsts_max_age.is_some(), "server started");

    let options = Options {
        web: source,
        hsts_max_age,
        header_timeout: settings.header_timeout,
        data_folder: data.root().to_path_buf(),
        trusted_proxies: settings.trusted_proxies,
        limits: settings.limits,
        auth: wired.auth,
        probe: wired.probe,
    };
    // A certificate that runs out while the server runs is renewed without a restart (M1-F03).
    let renewal = certificate.map(|served| {
        renewal::spawn(
            served,
            || jiff::Timestamp::now().as_second(),
            renewal::CHECK_EVERY,
        )
    });
    let served = bb_api::serve(incoming, options, shutdown).await;
    if let Some(timer) = renewal {
        timer.abort();
    }
    served?;
    tracing::info!("server stopped");
    Ok(())
}

/// How long browsers are told to insist on HTTPS: a day with a certificate Bugaboo made (a
/// person who trusts it for `localhost` must not pin their other local tools), a year with a
/// provided certificate or behind a trusted proxy, nothing for plain HTTP on this computer.
fn hsts_max_age(settings: &Settings) -> Option<u64> {
    match settings.tls {
        TlsMode::SelfSigned => Some(HSTS_MAX_AGE_SELF_SIGNED),
        TlsMode::Files { .. } => Some(HSTS_MAX_AGE_TRUSTED),
        TlsMode::Off => (!settings.trusted_proxies.is_empty()).then_some(HSTS_MAX_AGE_TRUSTED),
    }
}

/// What to tell about the setup code once the server listens.
enum SetupNotice {
    /// An administrator exists: there is nothing to tell.
    Nothing,
    /// A console is there: the code is printed on it, and only there (never in the log file,
    /// because the code is a secret until it is used).
    Console(String),
    /// Without a console (a service) the code is saved in a file in the data folder.
    Saved(Message),
    /// The file could not be written: where to look instead.
    NotSaved(Message),
}

/// Makes the setup code of the first start ready to be found (AUTH-09). This happens before the
/// server listens, so that an open port means a server that can be set up, for people and for
/// tests: a file with the code (private to the account that runs Bugaboo and, on Windows, the
/// administrators of the computer) is in place by then. The code is dead once the first
/// administrator exists; the file goes away with the change that makes the first administrator
/// (a port of `bb-app`, follow-up M1-F58), and here at the next start as a safety net.
fn prepare_setup_code(data: &DataFolder, code: Option<&str>, console: bool) -> SetupNotice {
    let file = data.setup_code_file();
    let Some(code) = code else {
        // An administrator exists: a code from an earlier start is dead, and so is its file.
        let _ = std::fs::remove_file(&file);
        return SetupNotice::Nothing;
    };
    if console {
        let _ = std::fs::remove_file(&file);
        return SetupNotice::Console(code.to_owned());
    }
    match save_setup_code(&file, code) {
        Ok(saved) => SetupNotice::Saved(saved),
        Err(failed) => SetupNotice::NotSaved(failed),
    }
}

/// Says how to find the setup code, once the server listens.
fn announce_setup_code(notice: &SetupNotice) {
    match notice {
        SetupNotice::Nothing => {}
        SetupNotice::Console(code) => {
            output::line(&message::server_setup_code(code).render());
            output::line(&Message::from(message::SERVER_SETUP_CODE_HINT).render());
        }
        SetupNotice::Saved(saved) => output::line(&saved.render()),
        SetupNotice::NotSaved(failed) => {
            tracing::warn!("the setup code could not be saved in a file");
            output::warning(&failed.render());
        }
    }
}

/// Saves the setup code in `file` for its owner only, and says where it is. The message of a
/// failure says where to look instead; it never contains the code.
fn save_setup_code(file: &Path, code: &str) -> Result<Message, Message> {
    write_private(file, format!("{code}\n").as_bytes(), Existing::Replace)
        .map(|()| message::server_setup_code_file(file.display()))
        .map_err(|error| message::server_setup_code_file_failed(file.display(), error))
}

/// What the listener does with connections: HTTPS with the configured certificate, or plain
/// HTTP that only trusted proxies may use. With HTTPS also the certificate in use, which the
/// server keeps current while it runs.
fn listener_options(
    settings: &Settings,
    data: &DataFolder,
) -> anyhow::Result<(ListenerOptions, Option<Arc<ServedCertificate>>)> {
    let source = match &settings.tls {
        TlsMode::Off => {
            let options = ListenerOptions {
                tls: None,
                only_from: (!settings.trusted_proxies.is_empty())
                    .then(|| settings.trusted_proxies.clone()),
                max_connections: settings.max_connections,
            };
            return Ok((options, None));
        }
        TlsMode::SelfSigned => TlsSource::SelfSigned {
            folder: data.tls(),
            names: settings.tls_names.clone(),
        },
        TlsMode::Files { cert, key } => {
            // Windows folders carry their own access lists; there is no mode to read.
            #[cfg(unix)]
            if let Some(warning) = key_readable_warning(key) {
                output::warning(&warning);
            }
            TlsSource::Files {
                cert: cert.clone(),
                key: key.clone(),
            }
        }
    };
    let served =
        serve_certificate(&source).map_err(|error| anyhow::anyhow!(tls_message(&error)))?;
    let options = ListenerOptions {
        tls: Some(served.config()),
        only_from: None,
        max_connections: settings.max_connections,
    };
    Ok((options, Some(Arc::new(served))))
}

/// A private key that other people on this Linux computer can read (group or world bits).
#[cfg(unix)]
fn key_readable_warning(key: &Path) -> Option<String> {
    use std::os::unix::fs::PermissionsExt as _;
    let mode = std::fs::metadata(key).ok()?.permissions().mode();
    (mode & 0o077 != 0).then(|| message::tls_key_readable(key.display()).render())
}

fn tls_message(error: &TlsError) -> String {
    let shown = |path: &Path| path.display().to_string();
    match error {
        TlsError::Read { path, source } => message::tls_read_failed(shown(path), source).render(),
        TlsError::Write { path, source } => message::tls_write_failed(shown(path), source).render(),
        TlsError::NoCertificate { path } => message::tls_no_certificate(shown(path)).render(),
        TlsError::NoKey { path } => message::tls_no_key(shown(path)).render(),
        TlsError::Unusable { reason } => message::tls_unusable(reason).render(),
        TlsError::Generate { reason } => message::tls_generate_failed(reason).render(),
    }
}

/// Which database is in use, without ever showing the address (it may hold a password).
fn database_line(url: Option<&str>) -> String {
    match url {
        None => Message::from(message::SERVER_DATABASE_BUILT_IN).render(),
        Some(url) if url.starts_with("sqlite:") => {
            Message::from(message::SERVER_DATABASE_SQLITE).render()
        }
        Some(_) => Message::from(message::SERVER_DATABASE_POSTGRES).render(),
    }
}

/// Completes when the system asks the server to stop: Ctrl+C, and on Unix SIGTERM (what
/// service managers send); on Windows also Ctrl+Break, closing the console and shutdown.
fn shutdown_signal() -> anyhow::Result<impl Future<Output = ()> + Send + 'static> {
    let failed = || Message::from(message::SERVER_SIGNAL_FAILED).render();
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut interrupt = signal(SignalKind::interrupt()).with_context(failed)?;
        let mut terminate = signal(SignalKind::terminate()).with_context(failed)?;
        Ok(async move {
            tokio::select! {
                _ = interrupt.recv() => {}
                _ = terminate.recv() => {}
            }
        })
    }
    #[cfg(windows)]
    {
        use tokio::signal::windows::{ctrl_break, ctrl_c, ctrl_close, ctrl_shutdown};
        let mut interrupt = ctrl_c().with_context(failed)?;
        let mut break_key = ctrl_break().with_context(failed)?;
        let mut close = ctrl_close().with_context(failed)?;
        let mut shutdown = ctrl_shutdown().with_context(failed)?;
        Ok(async move {
            tokio::select! {
                _ = interrupt.recv() => {}
                _ = break_key.recv() => {}
                _ = close.recv() => {}
                _ = shutdown.recv() => {}
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{self, Sources};

    #[cfg(unix)]
    #[test]
    fn sec_10_a_key_that_others_can_read_is_reported() {
        use std::os::unix::fs::PermissionsExt as _;
        let folder = tempfile::tempdir().unwrap();
        let key = folder.path().join("key.pem");
        std::fs::write(&key, "x").unwrap();
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o640)).unwrap();
        assert!(key_readable_warning(&key).is_some());
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(key_readable_warning(&key).is_none());
    }

    #[test]
    fn auth_09_a_setup_code_that_cannot_be_saved_is_reported_without_the_code() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("setup-code.txt");
        let saved = save_setup_code(&file, "k7q2-x9mf-3hd8-vt4w").unwrap();
        assert!(saved.render().contains("setup-code.txt"));
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "k7q2-x9mf-3hd8-vt4w\n"
        );
        let missing = folder.path().join("no-such-folder").join("setup-code.txt");
        let failed = save_setup_code(&missing, "k7q2-x9mf-3hd8-vt4w").unwrap_err();
        let text = failed.render();
        assert!(text.contains("no-such-folder"), "{text}");
        assert!(text.contains("bugaboo serve"), "{text}");
        assert!(!text.contains("k7q2"), "{text}");
    }

    /// Starts a server on a new data folder and returns what its setup code file held at the
    /// moment the first connection was accepted, read from another thread.
    async fn code_file_at_first_connection(
        folder: &Path,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // A port that is free now: nothing else in these tests binds one.
        let port = std::net::TcpListener::bind("127.0.0.1:0")?
            .local_addr()?
            .port();
        let data = folder.join("data");
        let given = [
            ("port", port.to_string()),
            ("tls_mode", "off".to_owned()),
            ("data_dir", data.to_string_lossy().into_owned()),
        ];
        let settings = config::resolve(&Sources {
            file: None,
            env: &|_| None,
            cli: &given,
            base: folder,
        })?;
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let server = serve_settings(settings, WebSource::Embedded, false, async move {
            let _ = stopped.await;
        });
        // Another thread connects the moment the port opens, without waiting: the file must be
        // there already.
        let client = std::thread::spawn(move || {
            let started = std::time::Instant::now();
            while std::net::TcpStream::connect(("127.0.0.1", port)).is_err() {
                assert!(
                    started.elapsed().as_secs() < 30,
                    "the server did not listen"
                );
            }
            let code = std::fs::read_to_string(data.join("setup-code.txt"));
            let _ = stop.send(());
            code
        });
        server.await?;
        Ok(client.join().map_err(|_| "the client thread panicked")??)
    }

    #[tokio::test]
    async fn auth_09_the_setup_code_file_is_there_when_the_port_accepts_connections()
    -> Result<(), Box<dyn std::error::Error>> {
        // Several starts, because the moment between opening the port and saving the file, were
        // it there, is short.
        for _ in 0..6 {
            let folder = tempfile::tempdir()?;
            let code = code_file_at_first_connection(folder.path()).await?;
            assert!(!code.trim().is_empty(), "the file was empty");
        }
        Ok(())
    }

    #[test]
    fn sec_11_hsts_lifetime_follows_who_made_the_certificate() {
        let settings = |args: &[&'static str]| {
            let sources = args
                .iter()
                .map(|pair| pair.split_once('=').unwrap())
                .map(|(k, v)| (k, v.to_owned()))
                .collect::<Vec<_>>();
            config::resolve(&Sources {
                file: None,
                env: &|_| None,
                cli: &sources,
                base: Path::new("exe"),
            })
            .unwrap()
        };
        let own = settings(&["port=1"]);
        assert_eq!(hsts_max_age(&own), Some(HSTS_MAX_AGE_SELF_SIGNED));
        let local = settings(&["port=1", "tls_mode=off"]);
        assert_eq!(hsts_max_age(&local), None);
        let proxied = settings(&[
            "port=1",
            "tls_mode=off",
            "trusted_proxies=10.0.0.5",
            "public_url=https://bugaboo.example.test",
        ]);
        assert_eq!(hsts_max_age(&proxied), Some(HSTS_MAX_AGE_TRUSTED));
        let provided = settings(&["port=1", "tls_mode=files", "tls_cert=a", "tls_key=b"]);
        assert_eq!(hsts_max_age(&provided), Some(HSTS_MAX_AGE_TRUSTED));
    }

    #[test]
    fn ops_08_the_database_line_never_shows_the_address() {
        let line = database_line(Some("postgres://user:secret@db.example.test/bugaboo"));
        assert!(!line.contains("secret") && !line.contains("db.example"));
        assert!(database_line(Some("sqlite:data.db")).contains("SQLite"));
        assert!(database_line(None).contains("built-in"));
    }
}
