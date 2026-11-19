//! The one draft of a new access key (AUTH-07): what the person chose and when it ends, before
//! its secret. One key, a renewed key and the keys of several agents are drafted here.

use bb_core::access_key::{AGENT_KEY_CHOICE, AccessKey, KeyChoice};
use bb_core::id::{AccessKeyId, AgentId, ProjectId, UserId};
use bb_policy::Permission;
use jiff::Timestamp;

use super::access_keys::expiry;
use super::ctx::Ctx;
use super::error::AppError;
use super::store::{AccessKeyRecord, AuthSettings, AuthStore};
use bb_core::ErrorCode;

/// When a key made now stops working, from the moment the person typed (none for the default):
/// the one reading of the end of a key, for one key and for the keys of several agents.
///
/// # Errors
/// `request.invalid` for a moment that cannot be read or is not allowed.
pub(crate) fn key_end(
    now: Timestamp,
    picked: Option<&str>,
    settings: &AuthSettings,
) -> Result<Timestamp, AppError> {
    let picked = picked
        .map(str::parse::<Timestamp>)
        .transpose()
        .map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))?;
    expiry(
        now,
        picked,
        settings.access_key_default_days,
        settings.access_key_longest_days,
    )
}

/// A key to make, as the person chose it: the one draft of a new key, before its secret.
pub(crate) struct KeyWanted {
    /// What the key is called.
    pub(crate) name: String,
    /// *What it may do*.
    pub(crate) choice: KeyChoice,
    /// The single permissions, for *More options*.
    pub(crate) permissions: Vec<Permission>,
    /// The projects it is limited to.
    pub(crate) projects: Vec<ProjectId>,
    /// When it stops working.
    pub(crate) expires_at: Timestamp,
    /// The agent it belongs to, if any.
    pub(crate) agent: Option<AgentId>,
}

impl KeyWanted {
    /// A key of an agent: what the agent may do is its work, cut from its person
    /// (`Narrowing::agent`), so the key narrows nothing more.
    pub(crate) fn of_agent(agent: AgentId, name: String, expires_at: Timestamp) -> Self {
        Self {
            name,
            choice: AGENT_KEY_CHOICE,
            permissions: Vec::new(),
            projects: Vec::new(),
            expires_at,
            agent: Some(agent),
        }
    }

    /// The record of the key, made now for `owner`, with no secret yet (`a_try_at_the_key`
    /// draws it).
    pub(crate) fn draft<S: AuthStore>(self, ctx: &Ctx<'_, S>, owner: UserId) -> AccessKeyRecord {
        AccessKeyRecord {
            key: AccessKey {
                id: AccessKeyId::generate(&mut ctx.ids()),
                owner,
                name: self.name,
                prefix: String::new(),
                secret_hash: bb_core::security_log::Digest::new([0; 32]),
                choice: self.choice,
                created_at: ctx.now(),
                expires_at: self.expires_at,
                last_used_at: None,
                agent: self.agent,
            },
            permissions: self.permissions,
            projects: self.projects,
        }
    }
}
