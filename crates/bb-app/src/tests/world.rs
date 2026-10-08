//! One small server on the in-memory store, made the way the program makes it: the starting roles,
//! the setup code, and the first administrator. The tests of the use cases start from here.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use bb_auth::{CSRF_KEY_LENGTH, CsrfKey, Passkeys, PasswordHasher, RelyingParty};
use bb_core::id::UserId;
use jiff::Timestamp;

use super::fake::FakeStore;
use crate::access::{MakeStartingRoles, make_starting_roles};
use crate::auth::{
    AcceptInvitation, AppError, AuthServices, CompleteSetup, CreateInvitation, Ctx, IssueSetupCode,
    NoFailureLimit, RequestInfo, Started, accept_invitation, complete_setup, create_invitation,
    issue_setup_code,
};
use crate::context::Clock;

pub(crate) use bb_dev_words::accounts::{OTHER_PASSWORD, PASSWORD, PUBLIC_URL};
/// The start of the clock of the tests: 2027-01-15 08:00:00 UTC.
const START: i64 = 1_800_000_000;

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
        let services = AuthServices::new(
            store.clone(),
            clock.clone(),
            PasswordHasher::new().map_err(text)?,
            Some(Passkeys::new(&party).map_err(text)?),
            CsrfKey::from_bytes(zeroize::Zeroizing::new([7; CSRF_KEY_LENGTH])),
            Arc::new(NoFailureLimit),
            "Bugaboo".to_owned(),
        );
        Ok(Self {
            store,
            services,
            clock,
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

    /// The first start and the first administrator, Ada.
    pub(crate) async fn new() -> Result<Self, String> {
        let mut world = Self::bare()?;
        let ctx = world.ctx();
        make_starting_roles(&ctx, MakeStartingRoles)
            .await
            .map_err(text)?;
        let code = issue_setup_code(&ctx, IssueSetupCode)
            .await
            .map_err(text)?
            .ok_or("no setup code on a new store")?;
        let admin = complete_setup(
            &ctx,
            CompleteSetup {
                code: code.as_str().to_owned(),
                display_name: "Ada Admin".to_owned(),
                email: "ada@example.test".to_owned(),
                password: PASSWORD.to_owned(),
                previous_session: None,
            },
        )
        .await
        .map_err(text)?;
        drop(ctx);
        world.admin = admin;
        Ok(world)
    }

    /// The context of one request (a new one every time: nothing is kept between requests).
    pub(crate) fn ctx(&self) -> Ctx<'_, FakeStore> {
        self.services.ctx(RequestInfo::default())
    }

    /// A new person without a role, made through an invitation of the administrator, and signed in.
    pub(crate) async fn person(&self, email: &str) -> Result<Started, AppError> {
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
                display_name: "Bob Builder".to_owned(),
                password: PASSWORD.to_owned(),
                previous_session: None,
            },
        )
        .await
    }
}
