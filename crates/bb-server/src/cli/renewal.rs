//! Keeps the HTTPS certificate current while the server runs (SEC-10, follow-up M1-F03).
//!
//! A timer looks at the certificate every hour (so at least once a day, which is what the
//! 30 days of notice need). A generated certificate with under 30 days left is renewed, and an
//! own certificate whose files changed is read again; either way the new certificate is swapped
//! into the settings of the listener, so connections that start from then on get it, without a
//! restart. A pair of files that cannot be used is reported in the log and the old certificate
//! keeps serving until the next look finds something better.

use std::sync::Arc;
use std::time::Duration;

use bb_api::tls::{Renewal, ServedCertificate};
use tokio::task::JoinHandle;
use tokio::time::MissedTickBehavior;

/// How often the certificate is looked at.
pub(super) const CHECK_EVERY: Duration = Duration::from_hours(1);

/// Starts the timer. `now` gives the seconds since 1970 (the clock of the computer, or a stopped
/// one in a test). The caller stops the timer with [`JoinHandle::abort`] when the server stops.
pub(super) fn spawn(
    served: Arc<ServedCertificate>,
    now: impl Fn() -> i64 + Send + Sync + 'static,
    every: Duration,
) -> JoinHandle<()> {
    let now = Arc::new(now);
    tokio::spawn(async move {
        let mut ticks = tokio::time::interval(every);
        ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
        // The first tick comes at once, and the certificate was looked at when the server started.
        ticks.tick().await;
        loop {
            ticks.tick().await;
            let (served, now) = (Arc::clone(&served), Arc::clone(&now));
            // Reading files and making a key is blocking work.
            if tokio::task::spawn_blocking(move || check(&served, now()))
                .await
                .is_err()
            {
                tracing::error!("the look at the HTTPS certificate stopped unexpectedly");
            }
        }
    })
}

/// One look at the certificate, and what it found in the log.
fn check(served: &ServedCertificate, now: i64) {
    match served.renew(now) {
        Ok(Renewal::Kept) => {}
        Ok(Renewal::Renewed) => tracing::info!("the HTTPS certificate was renewed"),
        Ok(Renewal::Reloaded) => tracing::info!("the HTTPS certificate was read again"),
        Err(error) => tracing::warn!(
            cause = %error,
            "the HTTPS certificate could not be renewed; the old one is still in use"
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::atomic::{AtomicI64, Ordering};

    use bb_api::tls::{TlsSource, serve_certificate};
    use bb_api::{Incoming, ListenerOptions};
    use rustls::pki_types::pem::PemObject as _;
    use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
    use rustls::time_provider::TimeProvider;
    use rustls::{ClientConfig, RootCertStore};
    use tokio::net::{TcpListener, TcpStream};
    use tokio_rustls::TlsConnector;

    use bb_testing::Failure;

    use super::*;

    const DAY: i64 = 86_400;

    fn seconds_now() -> i64 {
        jiff::Timestamp::now().as_second()
    }

    /// The clock of a client that stands still at one moment.
    #[derive(Debug)]
    struct Moment(i64);

    impl TimeProvider for Moment {
        fn current_time(&self) -> Option<UnixTime> {
            Some(UnixTime::since_unix_epoch(Duration::from_secs(
                u64::try_from(self.0).unwrap_or_default(),
            )))
        }
    }

    /// Whether a client that trusts exactly the certificate in `trusted` can connect, when its
    /// clock says `at` (seconds since 1970): a certificate renewed with a clock that is moved on
    /// is not valid yet for a client with the real time.
    async fn connects_trusting(
        address: std::net::SocketAddr,
        trusted: &Path,
        at: i64,
    ) -> Result<bool, Failure> {
        let mut roots = RootCertStore::empty();
        roots.add(CertificateDer::from_pem_file(trusted)?)?;
        let config = ClientConfig::builder_with_details(
            Arc::new(rustls::crypto::ring::default_provider()),
            Arc::new(Moment(at)),
        )
        .with_safe_default_protocol_versions()?
        .with_root_certificates(roots)
        .with_no_client_auth();
        let tcp = TcpStream::connect(address).await?;
        Ok(TlsConnector::from(Arc::new(config))
            .connect(ServerName::try_from("localhost")?, tcp)
            .await
            .is_ok())
    }

    /// Waits until `condition` holds, or fails after a few seconds.
    async fn eventually(mut condition: impl FnMut() -> bool) -> Result<(), Failure> {
        for _ in 0..500 {
            if condition() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        Err("the certificate was not renewed in time".into())
    }

    #[tokio::test]
    async fn sec_10_a_certificate_near_its_end_is_renewed_without_a_restart() -> Result<(), Failure>
    {
        let folder = tempfile::tempdir()?;
        let served = Arc::new(serve_certificate(&TlsSource::SelfSigned {
            folder: folder.path().to_path_buf(),
            names: vec![],
        })?);
        let tcp = TcpListener::bind("127.0.0.1:0").await?;
        let address = tcp.local_addr()?;
        // The listener gets its settings once and keeps them until the end of the test.
        let _incoming = Incoming::start(
            tcp,
            ListenerOptions {
                tls: Some(served.config()),
                ..ListenerOptions::default()
            },
        );
        let certificate = bb_api::tls::certificate_path(folder.path());
        let old_copy = folder.path().join("old.pem");
        std::fs::copy(&certificate, &old_copy)?;
        assert!(connects_trusting(address, &old_copy, seconds_now()).await?);
        let first_end = served.expires_at().ok_or("no end")?;

        // The clock stands where it is: nothing is renewed.
        let clock = Arc::new(AtomicI64::new(seconds_now()));
        let reading = Arc::clone(&clock);
        let timer = spawn(
            Arc::clone(&served),
            move || reading.load(Ordering::SeqCst),
            Duration::from_millis(20),
        );
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(served.expires_at(), Some(first_end));

        // Eleven months pass: under 30 days are left, and the next look renews the certificate.
        clock.fetch_add(336 * DAY, Ordering::SeqCst);
        eventually(|| served.expires_at() != Some(first_end)).await?;
        timer.abort();
        assert!(served.expires_at().ok_or("no end")? > first_end + 300 * DAY);

        // The same listener, never restarted, now hands out the new certificate: the new file
        // is trusted, the old one no longer is.
        let later = clock.load(Ordering::SeqCst);
        assert!(connects_trusting(address, &certificate, later).await?);
        assert!(!connects_trusting(address, &old_copy, later).await?);
        Ok(())
    }

    #[tokio::test]
    async fn sec_10_an_own_certificate_that_changed_is_served_after_the_next_look()
    -> Result<(), Failure> {
        let folder = tempfile::tempdir()?;
        // Two certificates of the same kind that Bugaboo makes, used as the person's own.
        let make = |name: &str| -> Result<(std::path::PathBuf, std::path::PathBuf), Failure> {
            let side = folder.path().join(name);
            std::fs::create_dir(&side)?;
            bb_api::tls::server_config(&TlsSource::SelfSigned {
                folder: side.clone(),
                names: vec![],
            })?;
            Ok((
                bb_api::tls::certificate_path(&side),
                side.join("self-signed-key.pem"),
            ))
        };
        let (first_cert, first_key) = make("first")?;
        let (second_cert, second_key) = make("second")?;
        let (cert, key) = (
            folder.path().join("cert.pem"),
            folder.path().join("key.pem"),
        );
        std::fs::copy(&first_cert, &cert)?;
        std::fs::copy(&first_key, &key)?;

        let served = Arc::new(serve_certificate(&TlsSource::Files {
            cert: cert.clone(),
            key: key.clone(),
        })?);
        let tcp = TcpListener::bind("127.0.0.1:0").await?;
        let address = tcp.local_addr()?;
        let _incoming = Incoming::start(
            tcp,
            ListenerOptions {
                tls: Some(served.config()),
                ..ListenerOptions::default()
            },
        );
        let now = seconds_now();
        assert!(connects_trusting(address, &first_cert, now).await?);
        assert!(!connects_trusting(address, &second_cert, now).await?);

        // The person replaces the files; the timer finds the change and swaps.
        std::fs::copy(&second_cert, &cert)?;
        std::fs::copy(&second_key, &key)?;
        let timer = spawn(Arc::clone(&served), seconds_now, Duration::from_millis(20));
        let mut swapped = false;
        for _ in 0..500 {
            if connects_trusting(address, &second_cert, now).await? {
                swapped = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        timer.abort();
        assert!(swapped, "the new certificate was never served");
        assert!(!connects_trusting(address, &first_cert, now).await?);
        Ok(())
    }
}
