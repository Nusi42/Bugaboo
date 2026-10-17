//! One small server on the in-memory store, made the way the program makes it: the starting roles,
//! the setup code, and the first administrator. The tests of the use cases start from here.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use bb_auth::{CSRF_KEY_LENGTH, CsrfKey, Passkeys, PasswordHasher, RelyingParty};
use bb_core::id::UserId;
use jiff::Timestamp;

use super::fake::FakeStore;
use super::fake_files::FakeFiles;
use crate::access::{CreateStartingRoles, create_starting_roles};
use crate::auth::{
    AcceptInvitation, AppError, AuthServices, CompleteSetup, CreateInvitation, Ctx, IssueSetupCode,
    NoFailureLimit, RequestInfo, Started, accept_invitation, complete_setup, create_invitation,
    issue_setup_code,
};
use crate::context::Clock;
use crate::editing::EditableRecords;
use crate::first_start::first_start;
use crate::mailer::FakeMailer;

pub(crate) use bb_dev_words::accounts::{CLOCK_START, OTHER_PASSWORD, PASSWORD, PUBLIC_URL};
/// The start of the clock of the tests: 2027-01-15 08:00:00 UTC.
const START: i64 = CLOCK_START;

/// A clock the test moves by hand.
#[derive(Debug)]
pub(crate) struct TestClock(AtomicI64);

impl TestClock {
    pub(crate) fn advance(&self, seconds: i64) {
        self.0.fetch_add(seconds, Ordering::SeqCst);
    }
}

impl Clock for TestClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_second(self.0.load(Ordering::SeqCst)).unwrap_or_default()
    }
}

/// The server, the store behind it, and the administrator who set it up.
pub(crate) struct World {
    pub(crate) store: FakeStore,
    pub(crate) services: AuthServices<FakeStore>,
    pub(crate) clock: Arc<TestClock>,
    /// The bytes of files.
    pub(crate) files: FakeFiles,
    /// What the server would have sent by e-mail.
    pub(crate) mailer: FakeMailer,
    pub(crate) admin: Started,
}

/// What a failure of a test says: a text.
pub(crate) type Outcome = Result<(), Box<dyn std::error::Error + Send + Sync>>;

fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}

impl World {
    /// The services on a store that nobody has used: no roles, no setup code, no people.
    pub(crate) fn bare() -> Result<Self, String> {
        let store = FakeStore::default();
        let clock = Arc::new(TestClock(AtomicI64::new(START)));
        let party = RelyingParty::from_public_url(PUBLIC_URL, "Bugaboo").map_err(text)?;
        let files = FakeFiles::default();
        let mailer = FakeMailer::default();
        let services = AuthServices::new(
            store.clone(),
            clock.clone(),
            PasswordHasher::new().map_err(text)?,
            Some(Passkeys::new(&party).map_err(text)?),
            CsrfKey::from_bytes(zeroize::Zeroizing::new([7; CSRF_KEY_LENGTH])),
            Arc::new(NoFailureLimit),
            "Bugaboo".to_owned(),
        )
        .with_files(Arc::new(files.clone()))
        .with_mailer(Arc::new(mailer.clone()))
        .with_public_url(PUBLIC_URL);
        Ok(Self {
            store,
            services,
            clock,
            files,
            mailer,
            // Replaced by `new`; a bare world has nobody.
            admin: Started {
                session_token: String::new(),
                csrf_token: String::new(),
                lifetime: std::time::Duration::ZERO,
                person: crate::auth::PersonView {
                    id: UserId::from_random([0; 16]),
                    display_name: String::new(),
                    email: String::new(),
                },
                second_step_needed: None,
            },
        })
    }

    /// The first start (`first_start`, as the server runs it), and the first administrator, Ada.
    pub(crate) async fn new() -> Result<Self, String> {
        let mut world = Self::bare()?;
        let code = first_start(&world.services)
            .await
            .map_err(text)?
            .ok_or("no setup code on a new store")?;
        world.admin = world.first_administrator(code.as_str()).await?;
        Ok(world)
    }

    /// The first start and the first administrator, before the issue types are made. It does the
    /// steps of `first_start` by hand and leaves out the issue types and the kinds of relation,
    /// for the tests of a server that has none yet.
    pub(crate) async fn new_without_issue_types() -> Result<Self, String> {
        let mut world = Self::bare()?;
        let ctx = world.ctx();
        create_starting_roles(&ctx, CreateStartingRoles)
            .await
            .map_err(text)?;
        let code = issue_setup_code(&ctx, IssueSetupCode)
            .await
            .map_err(text)?
            .ok_or("no setup code on a new store")?;
        drop(ctx);
        world.admin = world.first_administrator(code.as_str()).await?;
        Ok(world)
    }

    /// Ada, the first administrator, made with the setup code.
    async fn first_administrator(&self, code: &str) -> Result<Started, String> {
        complete_setup(
            &self.ctx(),
            CompleteSetup {
                code: code.to_owned(),
                display_name: "Ada Admin".to_owned(),
                email: "ada@example.test".to_owned(),
                password: PASSWORD.to_owned(),
                previous_session: None,
            },
        )
        .await
        .map_err(text)
    }

    /// The same server that also knows another kind of record.
    pub(crate) fn with_records(mut self, records: Arc<dyn EditableRecords>) -> Self {
        self.services = self.services.with_records(records);
        self
    }

    /// The context of one request (a new one every time: nothing is kept between requests).
    pub(crate) fn ctx(&self) -> Ctx<'_, FakeStore> {
        self.services.ctx(RequestInfo::default())
    }

    /// A new person without a role, made through an invitation of the administrator, and signed in.
    pub(crate) async fn person(&self, email: &str) -> Result<Started, AppError> {
        self.person_named(email, "Bob Builder").await
    }

    /// The same for a person with a name of their own.
    pub(crate) async fn person_named(
        &self,
        email: &str,
        display_name: &str,
    ) -> Result<Started, AppError> {
        let created = create_invitation(
            &self.ctx(),
            CreateInvitation {
                session_token: self.admin.session_token.clone(),
                email: email.to_owned(),
            },
        )
        .await?
        .into_value();
        let secret = created
            .link_path
            .split_once('#')
            .map(|(_, secret)| secret.to_owned())
            .unwrap_or_default();
        accept_invitation(
            &self.ctx(),
            AcceptInvitation {
                secret,
                display_name: display_name.to_owned(),
                password: PASSWORD.to_owned(),
                previous_session: None,
            },
        )
        .await
    }
}
