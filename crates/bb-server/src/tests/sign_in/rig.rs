//! One complete server without a socket, for the tests of the sign-in flows.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use bb_api::FailureLimits;
use bb_app::access::Permission;
use bb_app::access::{MakeStartingRoles, PolicyStore as _, make_starting_roles};
use bb_app::auth::{
    AcceptInvitation, AppError, AuthApi, AuthServices, AuthStore as _, AuthenticateQuery,
    CompleteSetup, CreateInvitation, IssueSetupCode, RequestInfo, SignIn, Started,
    issue_setup_code,
};
use bb_app::context::Clock;
use bb_auth::{CsrfKey, Passkeys, PasswordHasher, RelyingParty};
use bb_core::ErrorCode;
use bb_core::id::UserId;
use bb_store::{AuthStorage, Database, LogSeal};
use bb_testing::{TestDatabase, TestResult};
use jiff::Timestamp;

/// The public address the passkeys of the tests belong to.
pub(super) const PUBLIC_URL: &str = "https://bugaboo.test:47200";
/// A password that is not on the list of breached ones and has enough characters.
pub(super) const PASSWORD: &str = "Tr4mpoline-Quiet-Harbor-81";
/// Another one, for changing it.
pub(super) const OTHER_PASSWORD: &str = "Violet-Lantern-Marsh-5520";
/// The start of the clock of the tests: 2027-01-15 08:00:00 UTC.
const START: i64 = 1_800_000_000;

/// A clock the test moves by hand.
#[derive(Debug)]
pub(super) struct TestClock(AtomicI64);

impl TestClock {
    pub(super) fn advance(&self, seconds: i64) {
        self.0.fetch_add(seconds, Ordering::SeqCst);
    }
}

impl Clock for TestClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_second(self.0.load(Ordering::SeqCst)).unwrap_or_default()
    }
}

/// A server built from the real parts, on the test database.
pub(super) struct Rig {
    pub(super) database: Database,
    pub(super) storage: AuthStorage,
    pub(super) services: Arc<AuthServices<AuthStorage>>,
    pub(super) clock: Arc<TestClock>,
    pub(super) seal: LogSeal,
    /// The setup code of the first start, as the console would show it.
    pub(super) setup_code: String,
    /// Keeps the key files and the anchor file of the log alive.
    pub(super) folder: tempfile::TempDir,
}

impl Rig {
    /// A server with `failures` failed tries allowed per account and minute.
    pub(super) async fn start(
        db: &TestDatabase,
        failures: u32,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::start_with(db, failures, true).await
    }

    /// The same, with or without passkeys (a server reached by an IP address has none).
    pub(super) async fn start_with(
        db: &TestDatabase,
        failures: u32,
        passkeys: bool,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let rig = Self::build(db, failures, passkeys, true).await?;
        if rig.setup_code.is_empty() {
            return Err("no setup code on a new database".into());
        }
        Ok(rig)
    }

    /// A server on a database that somebody set up already (an upgraded one): no setup code.
    pub(super) async fn start_upgraded(
        db: &TestDatabase,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::build(db, 10, true, true).await
    }

    /// A server on a new database that has not had its first start: no starting roles, no setup
    /// code. The test makes the first start itself (two servers starting together).
    pub(super) async fn start_unstarted(
        db: &TestDatabase,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::build(db, 10, true, false).await
    }

    async fn build(
        db: &TestDatabase,
        failures: u32,
        passkeys: bool,
        first_start: bool,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let database = Database::connect(db.url()).await?;
        database.migrate(Timestamp::from_second(START)?).await?;
        let folder = tempfile::tempdir()?;
        std::fs::create_dir_all(folder.path().join("logs"))?;
        let keys = Arc::new(database.open_keys(&folder.path().join("keys")).await?);
        let seal = LogSeal::new(&keys, folder.path().join("logs").join("security-head.log"));
        let csrf = CsrfKey::from_bytes(keys.csrf_key().to_bytes());
        let storage = AuthStorage::new(database.clone(), keys, seal.clone());
        let clock = Arc::new(TestClock(AtomicI64::new(START)));
        let party = RelyingParty::from_public_url(PUBLIC_URL, "Bugaboo")?;
        let services = Arc::new(AuthServices::new(
            storage.clone(),
            clock.clone(),
            PasswordHasher::new()?,
            if passkeys {
                Some(Passkeys::new(&party)?)
            } else {
                None
            },
            csrf,
            FailureLimits::new(failures),
            "Bugaboo".to_owned(),
        ));
        // The first start, as the wiring does it: the starting roles first, then the setup code.
        let code = if first_start {
            make_starting_roles(&services.ctx(RequestInfo::default()), MakeStartingRoles)
                .await
                .map_err(|error| error.to_string())?;
            issue_setup_code(&services.ctx(RequestInfo::default()), IssueSetupCode)
                .await
                .map_err(|error| error.to_string())?
        } else {
            None
        };
        Ok(Self {
            database,
            storage,
            services,
            clock,
            seal,
            setup_code: code
                .map(|code| code.as_str().to_owned())
                .unwrap_or_default(),
            folder,
        })
    }

    /// The use cases as the HTTP layer sees them.
    pub(super) fn api(&self) -> &dyn AuthApi {
        &*self.services
    }

    /// A request from nowhere in particular.
    pub(super) fn info() -> RequestInfo {
        RequestInfo::default()
    }

    /// Makes the first administrator with the setup code.
    pub(super) async fn admin(&self, email: &str) -> Result<Started, AppError> {
        self.api()
            .complete_setup(
                Self::info(),
                CompleteSetup {
                    code: self.setup_code.clone(),
                    display_name: "Ada Admin".to_owned(),
                    email: email.to_owned(),
                    password: PASSWORD.to_owned(),
                    previous_session: None,
                },
            )
            .await
    }

    /// Makes a person with an invitation from the administrator behind `admin_session`.
    pub(super) async fn person(
        &self,
        admin_session: &str,
        email: &str,
    ) -> Result<Started, AppError> {
        let created = self
            .api()
            .create_invitation(
                Self::info(),
                CreateInvitation {
                    session_token: admin_session.to_owned(),
                    email: email.to_owned(),
                },
            )
            .await?
            .into_value();
        self.api()
            .accept_invitation(
                Self::info(),
                AcceptInvitation {
                    secret: secret_of(&created.link_path),
                    display_name: "Bob Builder".to_owned(),
                    password: PASSWORD.to_owned(),
                    previous_session: None,
                },
            )
            .await
    }

    /// Signs in with e-mail address and password.
    pub(super) async fn sign_in(&self, email: &str, password: &str) -> Result<Started, AppError> {
        self.api()
            .sign_in(
                Self::info(),
                SignIn {
                    email: email.to_owned(),
                    password: password.to_owned(),
                    previous_session: None,
                },
            )
            .await
    }

    /// What the person behind a session may do for the whole organization.
    pub(super) async fn permissions(
        &self,
        session: &str,
    ) -> Result<Vec<Permission>, Box<dyn std::error::Error + Send + Sync>> {
        let me = self
            .api()
            .authenticate(
                Self::info(),
                AuthenticateQuery {
                    session_token: session.to_owned(),
                },
            )
            .await
            .map_err(|error| error.to_string())?;
        Ok(me.access.permissions().iter().copied().collect())
    }

    /// How many people can manage roles for the whole organization (setup is open while it is 0).
    pub(super) async fn role_managers(
        &self,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let state = self.storage.organization_state().await?;
        Ok(bb_app::access::guards::role_managers(&state).len())
    }

    /// The person with this address.
    pub(super) async fn person_id(
        &self,
        email: &str,
    ) -> Result<UserId, Box<dyn std::error::Error + Send + Sync>> {
        let account = self
            .storage
            .account_by_email(email)
            .await?
            .ok_or("no such person")?;
        Ok(account.person.id)
    }
}

/// The secret in the link of an invitation (after the `#`).
pub(super) fn secret_of(link_path: &str) -> String {
    link_path
        .split_once('#')
        .map(|(_, secret)| secret.to_owned())
        .unwrap_or_default()
}

/// The error code of a refused request.
pub(super) fn refused<T>(result: Result<T, AppError>) -> Option<ErrorCode> {
    match result.err() {
        Some(AppError::Refused(code)) => Some(code),
        _ => None,
    }
}

/// Runs a test body that returns a plain error.
pub(super) type Outcome = TestResult;
