//! Makes the organization: the first administrator with the setup code, everybody else as that
//! administrator, by invitation, through the use cases the API calls. Every change therefore has
//! its event and its security log entry, and nothing is written to the database by hand.
//!
//! The generator never reads the clock. Every date comes from the one start moment it is given:
//! the people join in the minutes before it, one every [`STEP_SECONDS`].

use std::fmt;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;

use bb_app::auth::{
    AcceptInvitation, AuthApi, AuthServices, AuthStore as _, CompleteSetup, ConfirmCodeApp,
    CreateInvitation, IssueSetupCode, RequestInfo, SignOut, StartCodeApp, Started,
    issue_setup_code,
};
use bb_app::context::Clock as _;
use bb_auth::{CsrfKey, PasswordHasher, TotpSecret};
use bb_core::id::UserId;
use bb_core::message::{self, Message};
use bb_store::{
    AuthStorage, DATABASE_FILE, Database, KEYS_FOLDER, LOGS_FOLDER, LogSeal,
    SECURITY_LOG_ANCHOR_FILE, Verification,
};
use jiff::Timestamp;

use crate::clock::SampleClock;
use crate::error::SampleError;
use crate::limits::Unlimited;
use crate::names::NameList;
use crate::password::random_password;
use crate::roster::{SamplePerson, roster};

/// The seed of the sample when nobody names another one.
pub const DEFAULT_SEED: u64 = 42;

/// Seconds between two people joining. Short enough that the administrator's session never runs
/// out of idle time while the sample is made (30 minutes by default).
pub const STEP_SECONDS: i64 = 20;

/// What the sample is made from.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Decides the people and their order.
    pub seed: u64,
    /// The moment the sample ends. Everything is dated before it.
    pub start: Timestamp,
}

/// Where the database is.
#[derive(Clone, Debug)]
pub enum Target {
    /// The built-in database: a file in the data folder.
    BuiltIn,
    /// An empty database at an address (`postgres://…`, or `sqlite:…` for a file elsewhere). The
    /// address can hold a password, so it never shows in a message.
    Address(String),
}

/// A person that was made, with the ways to sign in as them.
pub struct MadePerson {
    /// The plan the person was made from.
    pub person: SamplePerson,
    /// The ID the database gave the person (random by design).
    pub id: UserId,
    /// The random password of this run.
    pub password: String,
    /// The secret of the code app, for people who have one (base 32, as an app takes it).
    pub code_app_secret: Option<String>,
}

// Passwords and secrets never show in a message or a failed test.
impl fmt::Debug for MadePerson {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MadePerson")
            .field("person", &self.id)
            .finish_non_exhaustive()
    }
}

/// What a step after the people can use: the use cases, signed in as the first administrator.
pub struct World<'a> {
    /// The use cases of signing in and inviting; later packages add their own services here.
    pub services: &'a AuthServices<AuthStorage>,
    /// The session of the administrator who makes the organization (the secret of its cookie).
    pub administrator_session: &'a str,
    /// The people, in the order they were made.
    pub people: &'a [MadePerson],
    /// The clock of the sample; set it before a step to date what the step makes.
    pub clock: &'a SampleClock,
}

/// EXTENSION POINT (M2 WP-13): what comes after the people. WP-13 fills it in with the groups,
/// the roles of the roster, the projects and the assignments, made through the use cases of
/// WP-05, WP-08 and WP-11. Until then [`NothingMore`] stands in. A step runs once, after every
/// person exists and before the administrator signs out.
pub trait Later {
    /// Makes whatever follows the people.
    fn after_people(
        &mut self,
        world: &World<'_>,
    ) -> impl Future<Output = Result<(), SampleError>> + Send;
}

/// The step that follows the people today: none.
#[derive(Clone, Copy, Debug, Default)]
pub struct NothingMore;

impl Later for NothingMore {
    async fn after_people(&mut self, _world: &World<'_>) -> Result<(), SampleError> {
        Ok(())
    }
}

/// A finished sample: its people and the open database, to dump and check.
pub struct Sample {
    pub(crate) options: Options,
    pub(crate) people: Vec<MadePerson>,
    pub(crate) database: Database,
    pub(crate) storage: AuthStorage,
    pub(crate) seal: LogSeal,
}

impl Sample {
    /// The people, in the order they were made.
    #[must_use]
    pub fn people(&self) -> &[MadePerson] {
        &self.people
    }

    /// Checks the whole security log, as `bugaboo security-log verify` does.
    ///
    /// # Errors
    /// [`SampleError::LogBroken`] when an entry does not verify, and the errors of the database.
    pub async fn verify_security_log(&self) -> Result<u64, SampleError> {
        match self.database.verify_security_log(&self.seal).await? {
            Verification::Valid {
                entries, anchored, ..
            } if anchored || entries == 0 => Ok(entries),
            Verification::Valid { .. } => Err(SampleError::LogBroken(
                "nothing vouches for the end of the log".to_owned(),
            )),
            Verification::Broken { seq, reason } => {
                Err(SampleError::LogBroken(format!("{reason:?} at entry {seq}")))
            }
        }
    }

    /// The ways to sign in, as the text of `sign-in.txt`: the only place where passwords and
    /// secrets are written.
    #[must_use]
    pub fn sign_in_text(&self) -> String {
        let mut text = String::new();
        for line in Message::from(message::SAMPLE_SIGN_IN_HEADER)
            .render()
            .lines()
        {
            text.push_str("# ");
            text.push_str(line);
            text.push('\n');
        }
        for made in &self.people {
            text.push_str(&made.person.email);
            text.push('\t');
            text.push_str(&made.password);
            text.push('\t');
            text.push_str(made.person.role.code());
            text.push('\t');
            text.push_str(&made.person.name);
            if let Some(secret) = &made.code_app_secret {
                text.push('\t');
                text.push_str(secret);
            }
            text.push('\n');
        }
        text
    }

    /// Closes the database, so its file holds everything and can be copied.
    pub async fn close(self) {
        self.database.close().await;
    }
}

/// The address a person invited by `link_path` opens: the secret is after the `#`.
fn secret_of(link_path: &str) -> &str {
    link_path.split_once('#').map_or("", |(_, secret)| secret)
}

/// A request from nowhere in particular: the sample is made on the server's own computer.
fn request() -> RequestInfo {
    RequestInfo::default()
}

/// Makes the organization of `options` in `data_folder` (made when missing) on `target`.
///
/// # Errors
/// [`SampleError`] for a folder or database that fails and for a use case that refuses a step.
pub async fn make<L: Later>(
    options: &Options,
    data_folder: &Path,
    target: &Target,
    later: &mut L,
) -> Result<Sample, SampleError> {
    let plan = roster(options.seed, &NameList::bundled()?)?;
    let count = i64::try_from(plan.len()).unwrap_or(0);
    let first_moment = moment(options.start, -count * STEP_SECONDS)?;
    let clock = Arc::new(SampleClock::at(first_moment));

    let Opened {
        database,
        storage,
        seal,
        csrf,
    } = open(data_folder, target, first_moment).await?;
    let services = AuthServices::new(
        storage.clone(),
        clock.clone(),
        PasswordHasher::new().map_err(|error| SampleError::step("password hasher", &error))?,
        // Passkeys come later (AUTH-08, PRIV-04 and the sample of M3 on).
        None,
        csrf,
        Arc::new(Unlimited),
        Message::from(message::APP_NAME).render(),
    );
    let api: &dyn AuthApi = &services;

    let mut made: Vec<MadePerson> = Vec::with_capacity(plan.len());
    let mut administrator_session = String::new();
    for (at, person) in plan.into_iter().enumerate() {
        let index = i64::try_from(at).unwrap_or(0);
        clock.set(moment(first_moment, index * STEP_SECONDS)?);
        let password = random_password()?;
        let started = if at == 0 {
            first_administrator(&services, &person, &password).await?
        } else {
            invited(api, &administrator_session, &person, &password).await?
        };
        let code_app_secret = if person.code_app {
            Some(
                add_code_app(
                    api,
                    &storage,
                    &clock,
                    &started.session_token,
                    started.person.id,
                    &password,
                )
                .await?,
            )
        } else {
            None
        };
        if at == 0 {
            // The administrator stays signed in to make everybody else.
            administrator_session.clone_from(&started.session_token);
        } else {
            sign_out(api, started.session_token).await?;
        }
        made.push(MadePerson {
            person,
            id: started.person.id,
            password,
            code_app_secret,
        });
    }

    clock.set(options.start);
    later
        .after_people(&World {
            services: &services,
            administrator_session: &administrator_session,
            people: &made,
            clock: &clock,
        })
        .await?;
    sign_out(api, administrator_session).await?;

    Ok(Sample {
        options: *options,
        people: made,
        database,
        storage,
        seal,
    })
}

/// Makes the first administrator with the setup code.
async fn first_administrator(
    services: &AuthServices<AuthStorage>,
    person: &SamplePerson,
    password: &str,
) -> Result<Started, SampleError> {
    let code = issue_setup_code(&services.ctx(request()), IssueSetupCode)
        .await
        .map_err(|error| SampleError::step("setup code", &error))?
        .ok_or_else(|| SampleError::Step {
            step: "setup code",
            reason: "setup was closed already".to_owned(),
        })?;
    let api: &dyn AuthApi = services;
    api.complete_setup(
        request(),
        CompleteSetup {
            code: code.as_str().to_owned(),
            display_name: person.name.clone(),
            email: person.email.clone(),
            password: password.to_owned(),
            previous_session: None,
        },
    )
    .await
    .map_err(|error| SampleError::step("first administrator", &error))
}

/// Makes a person as the administrator does: an invitation, and the person accepting it.
async fn invited(
    api: &dyn AuthApi,
    administrator_session: &str,
    person: &SamplePerson,
    password: &str,
) -> Result<Started, SampleError> {
    let created = api
        .create_invitation(
            request(),
            CreateInvitation {
                session_token: administrator_session.to_owned(),
                email: person.email.clone(),
            },
        )
        .await
        .map_err(|error| SampleError::step("invitation", &error))?;
    api.accept_invitation(
        request(),
        AcceptInvitation {
            secret: secret_of(&created.link_path).to_owned(),
            display_name: person.name.clone(),
            password: password.to_owned(),
            previous_session: None,
        },
    )
    .await
    .map_err(|error| SampleError::step("accepting the invitation", &error))
}

/// Ends the session, so nobody of the sample is signed in when it is done.
async fn sign_out(api: &dyn AuthApi, session_token: String) -> Result<(), SampleError> {
    api.sign_out(request(), SignOut { session_token })
        .await
        .map(|_| ())
        .map_err(|error| SampleError::step("signing out", &error))
}

/// A moment `seconds` after (or, negative, before) `from`.
fn moment(from: Timestamp, seconds: i64) -> Result<Timestamp, SampleError> {
    Timestamp::from_second(from.as_second().saturating_add(seconds))
        .map_err(|error| SampleError::step("dates", &error))
}

/// The database with what the use cases need around it.
struct Opened {
    database: Database,
    storage: AuthStorage,
    seal: LogSeal,
    csrf: CsrfKey,
}

/// Opens (and migrates) the database and the keys in `data_folder`.
async fn open(
    data_folder: &Path,
    target: &Target,
    applied_at: Timestamp,
) -> Result<Opened, SampleError> {
    let keys_folder = data_folder.join(KEYS_FOLDER);
    let logs_folder = data_folder.join(LOGS_FOLDER);
    for folder in [data_folder, &keys_folder, &logs_folder] {
        std::fs::create_dir_all(folder).map_err(|error| SampleError::folder(folder, &error))?;
    }
    let database = match target {
        Target::BuiltIn => Database::connect_file(&data_folder.join(DATABASE_FILE)).await?,
        Target::Address(url) => Database::connect(url).await?,
    };
    database.migrate(applied_at).await?;
    let keys = Arc::new(database.open_keys(&keys_folder).await?);
    let seal = LogSeal::new(&keys, logs_folder.join(SECURITY_LOG_ANCHOR_FILE));
    let csrf = CsrfKey::from_bytes(keys.csrf_key().to_bytes());
    let storage = AuthStorage::new(database.clone(), keys, seal.clone());
    Ok(Opened {
        database,
        storage,
        seal,
        csrf,
    })
}

/// Sets up the code app of a person as the person would, and returns its secret.
async fn add_code_app(
    api: &dyn AuthApi,
    storage: &AuthStorage,
    clock: &SampleClock,
    session: &str,
    person: UserId,
    password: &str,
) -> Result<String, SampleError> {
    let step = "code app";
    let setup = api
        .start_code_app(
            request(),
            StartCodeApp {
                session_token: session.to_owned(),
                password: password.to_owned(),
            },
        )
        .await
        .map_err(|error| SampleError::step(step, &error))?;
    // The app and the server share the secret; the sample reads it back to be the app.
    let stored = storage
        .totp_secret(person)
        .await?
        .ok_or_else(|| SampleError::Step {
            step,
            reason: "the secret was not stored".to_owned(),
        })?;
    let code = TotpSecret::from_bytes(&stored)
        .code_at(clock.now())
        .map_err(|error| SampleError::step(step, &error))?;
    api.confirm_code_app(
        request(),
        ConfirmCodeApp {
            session_token: session.to_owned(),
            code,
            password: password.to_owned(),
        },
    )
    .await
    .map_err(|error| SampleError::step(step, &error))?;
    Ok(setup.secret)
}
