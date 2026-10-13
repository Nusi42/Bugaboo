//! One complete server without a socket, for the tests of the sign-in flows.

use bb_app::auth::AccountStore as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use bb_api::FailureLimits;
use bb_app::access::Permission;
use bb_app::access::{MakeStartingRoles, PolicyStore as _, make_starting_roles};
use bb_app::auth::{
    AcceptInvitation, AppError, AuthApi, AuthServices, AuthenticateQuery, CompleteSetup,
    CreateInvitation, IssueSetupCode, RequestInfo, SignIn, Started, issue_setup_code,
};
use bb_app::comments::CommentRecords;
use bb_app::context::Clock;
use bb_app::context::SystemIds;
use bb_app::editing::{ChainedRecords, EditableRecords};
use bb_app::issues::{MakeStartingIssueTypes, make_starting_issue_types};
use bb_app::relations::{MakeStartingRelationKinds, make_starting_relation_kinds};
use bb_auth::{CsrfKey, Passkeys, PasswordHasher, RelyingParty};
use bb_core::ErrorCode;
use bb_core::event::{Actor, Channel, DomainEvent, EntityKind, EntityRef};
use bb_core::id::{EntityId, EventId, GroupId, RoleId, UserId};
use bb_core::settings::SettingValue;
use bb_store::{AuthStorage, Database, LogSeal, SettingChange};
use bb_testing::{Failure, TestDatabase};
use jiff::Timestamp;

pub(crate) use bb_dev_words::accounts::{OTHER_PASSWORD, PASSWORD, PUBLIC_URL};
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

/// How many failed password tries per account and minute a rig allows before it makes the person
/// wait (SEC-14): a test that is not about the limit takes the default, one that runs into the
/// limit takes `Few`, one that must never run into it takes `Unreached`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tries {
    /// What the server ships with.
    Default,
    /// More than the default, for a test that fails to sign in many times on purpose.
    Roomy,
    /// A few, so the test reaches the limit quickly.
    Few,
    /// So many that the limit is never reached.
    Unreached,
    /// This many, for a test of one exact number.
    Exactly(u32),
}

impl Tries {
    const fn per_minute(self) -> u32 {
        match self {
            Self::Default => bb_api::DEFAULT_SIGN_IN_FAILURES_PER_MINUTE,
            Self::Roomy => 50,
            Self::Few => 3,
            Self::Unreached => 100_000,
            Self::Exactly(tries) => tries,
        }
    }
}

/// Whether the server has passkeys (a server reached by an IP address has none).
#[derive(Clone, Copy, PartialEq, Eq)]
enum PasskeySupport {
    Available,
    Missing,
}

/// Whether the first start (the starting roles and the setup code) was made.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FirstStart {
    Made,
    Left,
}

/// What a rig is made of.
#[derive(Clone, Copy)]
struct Shape {
    tries: Tries,
    passkeys: PasskeySupport,
    first_start: FirstStart,
}

impl Shape {
    const NEW: Self = Self {
        tries: Tries::Default,
        passkeys: PasskeySupport::Available,
        first_start: FirstStart::Made,
    };
}

impl Rig {
    /// A server with the default number of failed tries allowed.
    pub(super) async fn start(db: &TestDatabase) -> Result<Self, Failure> {
        Self::start_new(db, Shape::NEW).await
    }

    /// A server that allows `tries` failed tries per account and minute.
    pub(super) async fn start_with(db: &TestDatabase, tries: Tries) -> Result<Self, Failure> {
        Self::start_new(
            db,
            Shape {
                tries,
                ..Shape::NEW
            },
        )
        .await
    }

    /// A server without passkeys.
    pub(super) async fn start_without_passkeys(db: &TestDatabase) -> Result<Self, Failure> {
        let shape = Shape {
            passkeys: PasskeySupport::Missing,
            ..Shape::NEW
        };
        Self::start_new(db, shape).await
    }

    async fn start_new(db: &TestDatabase, shape: Shape) -> Result<Self, Failure> {
        let rig = Self::build(db, shape).await?;
        if rig.setup_code.is_empty() {
            return Err("no setup code on a new database".into());
        }
        Ok(rig)
    }

    /// A server on a database that somebody set up already (an upgraded one): no setup code.
    pub(super) async fn start_upgraded(db: &TestDatabase) -> Result<Self, Failure> {
        Self::build(db, Shape::NEW).await
    }

    /// A server on a new database that has not had its first start: no starting roles, no setup
    /// code. The test makes the first start itself (two servers starting together).
    pub(super) async fn start_unstarted(db: &TestDatabase) -> Result<Self, Failure> {
        let shape = Shape {
            first_start: FirstStart::Left,
            ..Shape::NEW
        };
        Self::build(db, shape).await
    }

    /// A server that also knows other kinds of record (the notes of the tests of editing).
    pub(super) async fn start_with_records(
        db: &TestDatabase,
        records: Arc<dyn EditableRecords>,
    ) -> Result<Self, Failure> {
        let rig = Self::build_with(db, Shape::NEW, Some(records)).await?;
        if rig.setup_code.is_empty() {
            return Err("no setup code on a new database".into());
        }
        Ok(rig)
    }

    async fn build(db: &TestDatabase, shape: Shape) -> Result<Self, Failure> {
        Self::build_with(db, shape, None).await
    }

    async fn build_with(
        db: &TestDatabase,
        shape: Shape,
        records: Option<Arc<dyn EditableRecords>>,
    ) -> Result<Self, Failure> {
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
        let services = AuthServices::new(
            storage.clone(),
            clock.clone(),
            PasswordHasher::new()?,
            match shape.passkeys {
                PasskeySupport::Available => Some(Passkeys::new(&party)?),
                PasskeySupport::Missing => None,
            },
            csrf,
            FailureLimits::new(shape.tries.per_minute()),
            "Bugaboo".to_owned(),
        );
        // The comments are always known as records; a test adds the kinds of its own.
        let comments: Arc<dyn EditableRecords> = Arc::new(CommentRecords::new(storage.clone()));
        let services = Arc::new(services.with_records(match records {
            Some(records) => Arc::new(ChainedRecords::new(vec![records, comments])),
            None => comments,
        }));
        // The first start, as the wiring does it: the starting roles first, then the setup code.
        let code = if shape.first_start == FirstStart::Made {
            make_starting_roles(&services.ctx(RequestInfo::default()), MakeStartingRoles)
                .await
                .map_err(|error| error.to_string())?;
            make_starting_issue_types(
                &services.ctx(RequestInfo::default()),
                MakeStartingIssueTypes,
            )
            .await
            .map_err(|error| error.to_string())?;
            make_starting_relation_kinds(
                &services.ctx(RequestInfo::default()),
                MakeStartingRelationKinds,
            )
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

    /// The moment on the clock of the test.
    pub(super) fn clock_now(&self) -> Timestamp {
        self.clock.now()
    }

    /// Sets a whole-number setting of the organization, the way the settings screen will.
    pub(super) async fn set_organization_setting(
        &self,
        key: &str,
        value: i64,
    ) -> Result<(), Failure> {
        self.put_setting(key, SettingValue::Integer(value)).await
    }

    /// Sets a yes-or-no setting of the organization.
    pub(super) async fn set_organization_switch(
        &self,
        key: &str,
        value: bool,
    ) -> Result<(), Failure> {
        self.put_setting(key, SettingValue::Bool(value)).await
    }

    async fn put_setting(&self, key: &str, value: SettingValue) -> Result<(), Failure> {
        let mut ids = SystemIds;
        let event = DomainEvent {
            id: EventId::generate(&mut ids)?,
            time: self.clock_now(),
            actor: Actor::System,
            channel: Channel::System,
            entity: EntityRef {
                kind: EntityKind::new("setting")?,
                id: EntityId::from_bytes(bb_auth::stable_id("setting", key))?,
            },
            changes: vec![],
            reason: None,
        };
        self.database
            .set_setting(
                &event,
                &SettingChange {
                    key: key.to_owned(),
                    scope: bb_core::settings::Scope::Organization,
                    value,
                    locked: false,
                },
            )
            .await?;
        Ok(())
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
        self.person_called(admin_session, email, "Bob Builder")
            .await
    }

    /// The same, for a person who gives this name when they accept.
    pub(super) async fn person_called(
        &self,
        admin_session: &str,
        email: &str,
        display_name: &str,
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
                    display_name: display_name.to_owned(),
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
    pub(super) async fn permissions(&self, session: &str) -> Result<Vec<Permission>, Failure> {
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
    pub(super) async fn role_managers(&self) -> Result<usize, Failure> {
        let state = self.storage.organization_state().await?;
        Ok(bb_app::access::guards::role_managers(&state).len())
    }

    /// The version a role has now, as the store holds it.
    pub(super) async fn role_version(&self, role: RoleId) -> Result<i64, Failure> {
        let found = self.storage.role(role).await?.ok_or("no such role")?;
        Ok(found.role.version)
    }

    /// The version a group has now, as the store holds it.
    pub(super) async fn group_version(&self, group: GroupId) -> Result<i64, Failure> {
        let found = self.storage.group(group).await?.ok_or("no such group")?;
        Ok(found.version)
    }

    /// The person with this address.
    pub(super) async fn person_id(&self, email: &str) -> Result<UserId, Failure> {
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

/// The error code of a refused request, or `None` when it was not refused. A wait (429) or a
/// failure of the server is no refusal with a code and no success either: it fails the test, so
/// that `None` always means that the request went through.
pub(super) fn refused<T>(result: Result<T, AppError>) -> Option<ErrorCode> {
    let error = result.err()?;
    assert!(
        error.code().is_some(),
        "{error:?} is neither a success nor a refusal with a code"
    );
    error.code()
}

/// The same for a helper that boxed the error of the use case on the way out.
pub(super) fn refused_boxed<T>(result: Result<T, Failure>) -> Option<ErrorCode> {
    refused(result.map_err(|error| match error.downcast::<AppError>() {
        Ok(app) => *app,
        Err(other) => panic!("{other:?} is not a refusal of a use case"),
    }))
}
