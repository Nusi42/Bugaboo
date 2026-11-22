//! Whether the link to a PostgreSQL server is open to somebody in between (SEC-16): a server
//! reached over the network with an `sslmode` that does not check the certificate of the database
//! server (sqlx asks for `prefer` when nothing is said) can be impersonated, and whoever sits in
//! between reads the password and every row. A loopback host and a Unix socket do not cross the
//! network.

use std::net::IpAddr;
use std::str::FromStr as _;

use sqlx::postgres::{PgConnectOptions, PgSslMode};

/// The query parameter of a PostgreSQL address that says how the link is protected.
const SSL_MODE: &str = "sslmode";
/// The value of [`SSL_MODE`] that checks the certificate of the server and its name.
const VERIFY_FULL: &str = "verify-full";

/// Whether the address names a PostgreSQL server.
fn is_postgres(url: &str) -> bool {
    url.starts_with("postgres:") || url.starts_with("postgresql:")
}

/// Whether the host of the options is this computer: a loopback address, `localhost` or the
/// folder of a Unix socket.
fn is_local(options: &PgConnectOptions) -> bool {
    let host = options.get_host();
    let host = host.trim_start_matches('[').trim_end_matches(']');
    options.get_socket().is_some()
        || host.starts_with('/')
        || host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

/// Whether the server reaches the PostgreSQL database at `url` over the network without checking
/// the certificate of the database server. A SQLite address, a local host and an address that
/// cannot be read are not.
#[must_use]
pub fn certificate_unchecked(url: &str) -> bool {
    if !is_postgres(url) {
        return false;
    }
    PgConnectOptions::from_str(url).is_ok_and(|options| {
        !is_local(&options)
            && !matches!(
                options.get_ssl_mode(),
                PgSslMode::VerifyFull | PgSslMode::VerifyCa
            )
    })
}

/// The address `url` with `sslmode=verify-full` added when it names a PostgreSQL server on another
/// computer and says nothing of `sslmode` itself; any other address as it is.
#[must_use]
pub fn with_checked_certificate(url: &str) -> String {
    let says_mode = url.split_once('?').is_some_and(|(_, query)| {
        query
            .split('&')
            .any(|pair| pair.split('=').next() == Some(SSL_MODE))
    });
    let remote = is_postgres(url)
        && PgConnectOptions::from_str(url).is_ok_and(|options| !is_local(&options));
    if !remote || says_mode {
        return url.to_owned();
    }
    let joint = if url.contains('?') { '&' } else { '?' };
    format!("{url}{joint}{SSL_MODE}={VERIFY_FULL}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sec_16_a_database_on_the_network_without_a_checked_certificate_is_named() {
        for open in [
            "postgres://bugaboo:secret@db.example.org/bugaboo",
            "postgres://db.example.org/bugaboo?sslmode=prefer",
            "postgres://db.example.org/bugaboo?sslmode=require",
            "postgresql://192.0.2.7:5433/bugaboo?sslmode=disable",
        ] {
            assert!(certificate_unchecked(open), "{open}");
        }
        for closed in [
            "postgres://db.example.org/bugaboo?sslmode=verify-full",
            "postgres://db.example.org/bugaboo?sslmode=verify-ca",
            "postgres://localhost/bugaboo",
            "postgres://127.0.0.1:5432/bugaboo",
            "postgres://[::1]/bugaboo",
            "postgres:///bugaboo?host=/run/postgresql",
            "sqlite:bugaboo.db",
            "not an address",
        ] {
            assert!(!certificate_unchecked(closed), "{closed}");
        }
    }

    #[test]
    fn sec_16_setup_asks_a_database_on_the_network_to_check_its_certificate() {
        assert_eq!(
            with_checked_certificate("postgres://u:p@db.example.org/bugaboo"),
            "postgres://u:p@db.example.org/bugaboo?sslmode=verify-full"
        );
        assert_eq!(
            with_checked_certificate("postgres://db.example.org/bugaboo?application_name=b"),
            "postgres://db.example.org/bugaboo?application_name=b&sslmode=verify-full"
        );
        // What the person said is kept, and a local server or SQLite is left alone.
        for kept in [
            "postgres://db.example.org/bugaboo?sslmode=require",
            "postgres://localhost/bugaboo",
            "sqlite:bugaboo.db",
        ] {
            assert_eq!(with_checked_certificate(kept), kept);
        }
        assert!(!certificate_unchecked(&with_checked_certificate(
            "postgres://db.example.org/bugaboo"
        )));
    }
}
