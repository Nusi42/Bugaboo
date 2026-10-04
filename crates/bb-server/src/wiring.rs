//! Wires the adapters to the use cases (architecture §4): opens the database, brings its schema up
//! to date, opens the key files, and builds the sign-in services around them. This is the only
//! place that knows both `bb-store` and `bb-app`.
//!
//! The order matters. The database is opened and migrated first, then the keys are opened with
//! it (`Database::open_keys` refuses to make new keys for a database that already holds secrets or
//! a security log), then the services are built, and last the setup code of the first start is
//! made, so a failure of any step stops the server before it listens.

use std::sync::Arc;

use anyhow::anyhow;
use bb_api::tls::certificate_path;
use bb_api::{AuthHandle, FailureLimits, ProbeHandle};
use bb_app::auth::{AuthServices, IssueSetupCode, RequestInfo, issue_setup_code};
use bb_app::context::SystemClock;
use bb_auth::{CsrfKey, Passkeys, PasswordHasher, RelyingParty};
use bb_core::message::{self, Message};
use bb_store::{AuthStorage, Database, LogSeal};

use crate::config::{Settings, TlsMode};
use crate::data_folder::DataFolder;
use crate::probe::{Probe, database_kind};

/// What the server needs from the wiring.
pub(crate) struct Wired {
    /// The sign-in use cases for the API.
    pub(crate) auth: AuthHandle,
    /// How the status page and `/health/ready` measure the server.
    pub(crate) probe: ProbeHandle,
    /// The setup code of the first start, to print once. `None` when an administrator exists.
    pub(crate) setup_code: Option<String>,
}

fn database_failed(error: &impl std::fmt::Display) -> anyhow::Error {
    anyhow!(message::server_database_failed(error).render())
}

/// Opens everything the sign-in routes need.
///
/// # Errors
/// Fails with a plain explanation when the database cannot be opened or migrated, when the key
/// files are missing from a database that needs them, or when the system gives no secure
/// randomness.
pub(crate) async fn wire(settings: &Settings, data: &DataFolder) -> anyhow::Result<Wired> {
    let database = match &settings.database_url {
        Some(url) => Database::connect(url).await,
        None => Database::connect_file(&data.database_file()).await,
    }
    .map_err(|error| database_failed(&error))?;
    let applied = database
        .migrate(jiff::Timestamp::now())
        .await
        .map_err(|error| database_failed(&error))?;
    tracing::info!(migrations = applied.len(), "database ready");
    let keys = Arc::new(
        database
            .open_keys(&data.keys())
            .await
            .map_err(|error| database_failed(&error))?,
    );
    let seal = LogSeal::new(&keys, data.security_log_anchor());
    let csrf = CsrfKey::from_bytes(keys.csrf_key().to_bytes());
    let certificate = match &settings.tls {
        TlsMode::SelfSigned => Some(certificate_path(&data.tls())),
        TlsMode::Files { cert, .. } => Some(cert.clone()),
        TlsMode::Off => None,
    };
    let probe = Probe::new(
        database.clone(),
        seal.clone(),
        database_kind(settings.database_url.as_deref()),
        certificate,
    );
    let storage = AuthStorage::new(database, keys, seal);

    let name = Message::from(message::APP_NAME).render();
    // Passkeys are tied to a host name; a server reached by an IP address cannot offer them.
    let passkeys = RelyingParty::from_public_url(&settings.public_url, &name)
        .and_then(|party| Passkeys::new(&party))
        .inspect_err(|error| {
            tracing::warn!(cause = %error, "passkeys are off: the public address cannot be used for them");
        })
        .ok();
    let services = AuthServices::new(
        storage,
        Arc::new(SystemClock),
        PasswordHasher::new().map_err(|error| anyhow!(error.to_string()))?,
        passkeys,
        csrf,
        FailureLimits::new(settings.limits.sign_in_failures_per_minute),
        name,
    );
    let setup_code = issue_setup_code(&services.ctx(RequestInfo::default()), IssueSetupCode)
        .await
        .map_err(|error| anyhow!(error.to_string()))?
        .map(|code| code.as_str().to_owned());
    Ok(Wired {
        auth: AuthHandle::new(Arc::new(services)),
        probe: ProbeHandle::new(Arc::new(probe)),
        setup_code,
    })
}

#[cfg(test)]
#[path = "tests/wiring.rs"]
mod tests;
