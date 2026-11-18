//! Wires the adapters to the use cases (architecture §4): opens the database, brings its schema up
//! to date, opens the key files, and builds the sign-in services around them. This is the only
//! place that knows both `bb-store` and `bb-app`.
//!
//! The order matters. The database is opened and migrated first, then the keys are opened with
//! it (`Database::open_keys` refuses to make new keys for a database that already holds secrets or
//! a security log), then the services are built, and last the setup code of the first start is
//! made, so a failure of any step stops the server before it listens.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::anyhow;
use bb_ai::HttpAiProvider;
use bb_api::tls::ServedCertificate;
use bb_api::{AuthHandle, FailureLimits, ProbeHandle};
use bb_app::auth::{AuthServices, RequestInfo};
use bb_app::context::SystemClock;
use bb_app::first_start::first_start;
use bb_app::ports::SetupCodeFile;
use bb_auth::{CsrfKey, Passkeys, PasswordHasher, RelyingParty};
use bb_core::message::{self, Message};
use bb_files::{DiskFiles, PreviewCommand};
use bb_notify::SmtpMailer;
use bb_store::{AuthStorage, Database, LogSeal};

use bb_app::backups::{BackupFile, BackupFiles};
use bb_app::ports::BoxFuture;
use bb_core::backup::name::{BackupName, Purpose};
use bb_core::backup::run::CreatedFile;
use bb_store::backup::backups_in;

use crate::cli::backup::{backup_folder, places};
use crate::config::Settings;
use crate::data_folder::{DataFolder, protect_key_files};
use crate::probe::{Probe, database_kind};

/// The subcommand of the program that creates a preview, which the server starts as a child
/// process (ADR-0032 §4).
pub(crate) const PREVIEW_COMMAND: &str = "preview";

/// What the caller of `wire` knows and the status page shows.
#[derive(Default)]
pub(crate) struct Serving {
    /// The certificate the server serves (and keeps current), when it speaks HTTPS itself.
    pub(crate) certificate: Option<Arc<ServedCertificate>>,
}

/// What the server needs from the wiring.
pub(crate) struct Wired {
    /// The sign-in use cases for the API.
    pub(crate) auth: AuthHandle,
    /// How the status page and `/health/ready` measure the server.
    pub(crate) probe: ProbeHandle,
    /// The MCP server for outside AI apps (AI-20).
    pub(crate) mcp: bb_mcp::McpService,
    /// The setup code of the first start, to print once. `None` when an administrator exists.
    pub(crate) setup_code: Option<String>,
    /// The services the background job worker runs with (`bb-jobs`).
    pub(crate) jobs: Arc<AuthServices<AuthStorage>>,
}

/// The setup code file of a server without a console, as the use case that makes the first
/// administrator sees it: something it can remove.
struct SetupCodeOnDisk(PathBuf);

impl SetupCodeFile for SetupCodeOnDisk {
    fn remove(&self) -> std::io::Result<()> {
        match std::fs::remove_file(&self.0) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }
}

fn database_failed(error: &impl std::fmt::Display) -> anyhow::Error {
    anyhow!(message::server_database_failed(error).render())
}

/// The backup files of the running server (OPS-07): created into the folder the setting names,
/// or the backups folder of the data folder.
pub(crate) struct ServerBackups {
    database: Database,
    data: DataFolder,
    keys: Arc<bb_store::KeyRing>,
}

impl ServerBackups {
    /// The backups of the installation in `data`, with its database and the keys that seal them.
    pub(crate) const fn new(
        database: Database,
        data: DataFolder,
        keys: Arc<bb_store::KeyRing>,
    ) -> Self {
        Self {
            database,
            data,
            keys,
        }
    }

    fn folder(&self, named: Option<&str>) -> std::io::Result<PathBuf> {
        backup_folder(&self.data, named.map(std::path::Path::new))
            .map_err(|error| std::io::Error::other(error.to_string()))
    }
}

impl BackupFiles for ServerBackups {
    fn create(
        &self,
        folder: Option<String>,
        purpose: Purpose,
        now: jiff::Timestamp,
    ) -> BoxFuture<'_, std::io::Result<CreatedFile>> {
        Box::pin(async move {
            let places = places(
                &self.data,
                self.folder(folder.as_deref())?,
                Arc::clone(&self.keys),
            );
            self.database
                .create_backup(&places, purpose, now)
                .await
                .map(|created| created.file)
                .map_err(|error| std::io::Error::other(error.to_string()))
        })
    }

    fn list(&self, folder: Option<String>) -> BoxFuture<'_, std::io::Result<Vec<BackupFile>>> {
        Box::pin(async move {
            let found = backups_in(&self.folder(folder.as_deref())?)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            Ok(found
                .into_iter()
                .map(|(name, _, size)| BackupFile { name, size })
                .collect())
        })
    }

    fn delete(
        &self,
        folder: Option<String>,
        name: BackupName,
    ) -> BoxFuture<'_, std::io::Result<()>> {
        Box::pin(async move {
            std::fs::remove_file(self.folder(folder.as_deref())?.join(name.to_string()))
        })
    }
}

/// The stored data of an installation that exists, opened by the commands that look after it
/// (`security-log verify`, `rotate-key`, `keys export`) and not by the server.
pub(crate) struct Store {
    pub(crate) database: Database,
    pub(crate) keys: Arc<bb_store::KeyRing>,
    pub(crate) seal: LogSeal,
}

/// Opens the database and the keys of an installation that exists. It changes nothing: no
/// migration, and no new keys for a database that has data (`Database::open_keys`).
///
/// # Errors
/// Fails with a plain explanation when the database cannot be opened or the key files are
/// missing.
pub(crate) async fn open_store(settings: &Settings, data: &DataFolder) -> anyhow::Result<Store> {
    let database = connect(settings, data).await?;
    let keys = Arc::new(
        database
            .open_keys(&data.keys())
            .await
            .map_err(|error| database_failed(&error))?,
    );
    let seal = LogSeal::new(&keys, data.security_log_anchor());
    Ok(Store {
        database,
        keys,
        seal,
    })
}

/// Opens the database the settings name: the address given, or the built-in database of the
/// data folder. Nothing is migrated.
pub(crate) async fn connect(settings: &Settings, data: &DataFolder) -> anyhow::Result<Database> {
    match &settings.database_url {
        Some(url) => Database::connect(url).await,
        None => Database::connect_file(&data.database_file()).await,
    }
    .map_err(|error| database_failed(&error))
}

/// Opens everything the sign-in routes need.
///
/// # Errors
/// Fails with a plain explanation when the database cannot be opened or migrated, when the key
/// files are missing from a database that needs them, or when the system gives no secure
/// randomness.
pub(crate) async fn wire(
    settings: &Settings,
    data: &DataFolder,
    serving: Serving,
) -> anyhow::Result<Wired> {
    let database = connect(settings, data).await?;
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
    // The master key gets an access list of its own on Windows (M1-F22), also a key file made
    // just now.
    protect_key_files(&data.keys()).map_err(|reason| {
        anyhow!(message::data_folder_protect_failed(data.keys().display(), reason).render())
    })?;
    let seal = LogSeal::new(&keys, data.security_log_anchor());
    let csrf = CsrfKey::from_bytes(keys.csrf_key().to_bytes());
    let storage = AuthStorage::new(database.clone(), Arc::clone(&keys), seal.clone());
    let name = Message::from(message::APP_NAME).render();
    if settings.public_url.is_empty() {
        // Without it an AI app is told the address the request came to, which a name pointed at this
        // machine can choose, and the page of another site is judged by that address too (ADR-0042).
        tracing::warn!("no public address is set: AI apps are told the address a request came to");
    }
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
    )
    .with_setup_code_file(Arc::new(SetupCodeOnDisk(data.setup_code_file())))
    .with_files(Arc::new(DiskFiles::new(
        data.files(),
        PreviewCommand::of_this_program(PREVIEW_COMMAND)
            .map_err(|error| anyhow!(error.to_string()))?,
    )))
    .with_backups(Arc::new(ServerBackups::new(
        database.clone(),
        DataFolder::existing(data.root())
            .ok_or_else(|| anyhow!(message::cli_no_data(data.root().display()).render()))?,
        Arc::clone(&keys),
    )))
    .with_mailer(Arc::new(SmtpMailer))
    .with_ai_provider(Arc::new(HttpAiProvider))
    .with_public_url(settings.public_url.clone())
    .with_mail_batch(settings.mail_batch);
    // A server that runs as a service writes its log to a file, which a job rotates (M1-F61).
    let services = match crate::log::server_log_file() {
        Some(file) => services.with_server_log(file),
        None => services,
    };
    let probe = Probe::new(
        database,
        seal,
        database_kind(settings.database_url.as_deref()),
        serving.certificate,
        services.log_writes(),
        data.root().to_path_buf(),
    );
    // The first start: the starting roles (and the Administrator role of the people who were
    // administrators in M1), the issue types and kinds of relation, then the setup code.
    let setup_code = first_start(&services)
        .await
        .map_err(|error| anyhow!(error.to_string()))?
        .map(|code| code.as_str().to_owned());
    // The jobs that come back every day (clearing old drafts, catching the history up) are queued
    // at every start; the job worker (`bb-jobs`) does them (ADR-0017, M3-F30, M3-F71).
    services
        .ctx(RequestInfo::default())
        .start_daily_jobs()
        .await
        .map_err(|error| anyhow!(error.to_string()))?;
    let services = Arc::new(services);
    let mcp = bb_mcp::router(bb_mcp::McpOptions {
        api: services.clone(),
    });
    Ok(Wired {
        jobs: Arc::clone(&services),
        auth: AuthHandle::new(services),
        mcp,
        probe: ProbeHandle::new(Arc::new(probe)),
        setup_code,
    })
}

#[cfg(test)]
#[path = "tests/wiring.rs"]
mod tests;
