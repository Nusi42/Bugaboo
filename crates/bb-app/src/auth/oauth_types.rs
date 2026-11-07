//! What the use cases of the sign-in of AI apps take and give back (AI-22, ADR-0042). Inputs that
//! change something are named after what they do; inputs that only read end in `Query`.

use std::fmt;

use bb_core::access_key::KeyChoice;
use bb_core::id::{GrantId, OAuthClientId, ProjectId};
use bb_core::oauth::OAuthProblem;
use bb_policy::Masked;
use jiff::Timestamp;
use zeroize::Zeroizing;

use crate::access::ProjectLabel;

/// An app registers itself (RFC 7591).
#[derive(Clone, Debug)]
pub struct RegisterApp {
    /// The name the app gives itself.
    pub name: String,
    /// The addresses it comes back to after a person said *Allow*.
    pub return_addresses: Vec<String>,
}

/// An app that was registered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegisteredApp {
    /// The ID the app is known by from now on.
    pub client_id: OAuthClientId,
    /// The name, as kept.
    pub name: String,
    /// The addresses, as kept.
    pub return_addresses: Vec<String>,
    /// When it registered.
    pub registered_at: Timestamp,
}

/// How a registration ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegisterOutcome {
    /// The app is registered.
    Registered(RegisteredApp),
    /// The app was turned away.
    Refused(OAuthProblem),
}

/// What an app asked for when it sent the person to the consent step (RFC 6749 §4.1.1, RFC 7636,
/// RFC 8707).
#[derive(Clone, Debug, Default)]
pub struct AuthorizationRequest {
    /// The ID of the app.
    pub client_id: String,
    /// `code`, the only kind of answer.
    pub response_type: String,
    /// The address to come back to.
    pub return_address: String,
    /// The PKCE challenge.
    pub challenge: String,
    /// `S256`, the only method.
    pub challenge_method: String,
    /// What the app asks to be allowed: `see-only` or `see-and-change`; empty means `see-only`.
    pub scope: String,
    /// The resource the token is for; empty means the MCP server.
    pub resource: String,
    /// What the app wants back unchanged.
    pub state: String,
}

/// Ask what an app wants, for the page that asks the person.
#[derive(Clone, Debug)]
pub struct AuthorizationQuery {
    /// The session of the person.
    pub session_token: String,
    /// What the app asked.
    pub request: AuthorizationRequest,
}

/// What the page shows before the person decides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationView {
    /// The name the app gave itself. Bugaboo cannot check it.
    pub app_name: String,
    /// Whether the app comes back to the person's own computer.
    pub on_this_computer: bool,
    /// The web address the app comes back to, for an app that does not come back to this computer:
    /// the person sees where the answer will go.
    pub web_address: Option<String>,
    /// The scope the app asked for; the person may narrow it, never widen it.
    pub asked: KeyChoice,
}

impl Masked for AuthorizationView {}

/// The person's answer to the consent step.
#[derive(Clone, Debug)]
pub struct DecideAuthorization {
    /// The session of the person.
    pub session_token: String,
    /// What the app asked.
    pub request: AuthorizationRequest,
    /// *Allow* or *Cancel*.
    pub allow: bool,
    /// What the person allows: at most what the app asked.
    pub scope: KeyChoice,
    /// The projects the grant counts in; none for every project the person can see.
    pub projects: Vec<ProjectId>,
}

/// Where the browser goes next: back to the app, with a code or with the refusal.
///
/// The address holds the code, so it is wiped when it is dropped and never written by `Debug`.
#[derive(Clone)]
pub struct AuthorizationAnswer {
    /// The address of the app with the answer in it.
    pub redirect_to: Zeroizing<String>,
}

impl fmt::Debug for AuthorizationAnswer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AuthorizationAnswer")
            .finish_non_exhaustive()
    }
}

impl Masked for AuthorizationAnswer {}

/// The app swaps a code or a renewing token for tokens (RFC 6749 §4.1.3, §6).
#[derive(Clone)]
pub struct TokenRequest {
    /// `authorization_code` or `refresh_token`.
    pub grant_type: String,
    /// The ID of the app.
    pub client_id: String,
    /// For a code: the code.
    pub code: Zeroizing<String>,
    /// For a code: the address the code was given for.
    pub return_address: String,
    /// For a code: the PKCE verifier.
    pub verifier: Zeroizing<String>,
    /// For a renewal: the renewing token.
    pub refresh_token: Zeroizing<String>,
    /// The resource the token is for; empty means the MCP server.
    pub resource: String,
}

impl fmt::Debug for TokenRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TokenRequest")
            .field("grant_type", &self.grant_type)
            .finish_non_exhaustive()
    }
}

/// The tokens an app is given. Shown once, never stored, never logged.
#[derive(Clone)]
pub struct TokenAnswer {
    /// The token to send with every call.
    pub access_token: Zeroizing<String>,
    /// Seconds the access token works.
    pub expires_in: i64,
    /// The token that renews it; the old one is dead from now.
    pub refresh_token: Zeroizing<String>,
    /// What the app may do.
    pub scope: KeyChoice,
}

impl fmt::Debug for TokenAnswer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TokenAnswer")
            .field("scope", &self.scope)
            .finish_non_exhaustive()
    }
}

/// How a swap of a code or a token ended.
#[derive(Clone, Debug)]
pub enum TokenOutcome {
    /// The app has tokens.
    Issued(TokenAnswer),
    /// The app was turned away.
    Refused(OAuthProblem),
}

/// List the apps a person allowed.
#[derive(Clone, Debug)]
pub struct ConnectedAppsQuery {
    /// The session of the person.
    pub session_token: String,
}

/// One app a person allowed, as *Connected apps* shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectedAppView {
    /// The ID to take it back with.
    pub id: GrantId,
    /// The name the app gave itself.
    pub name: String,
    /// Whether the app is on the person's own computer.
    pub on_this_computer: bool,
    /// What it may do.
    pub scope: KeyChoice,
    /// The projects it is limited to (those the person may not see are left out); empty for every
    /// project.
    pub projects: Vec<ProjectLabel>,
    /// When the person allowed it.
    pub allowed_at: Timestamp,
    /// The last time it made a call, to the minute.
    pub last_used_at: Option<Timestamp>,
}

/// The apps a person allowed, newest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectedApps {
    /// The apps.
    pub items: Vec<ConnectedAppView>,
}

impl Masked for ConnectedApps {}

/// A person ends what they allowed an app, at once.
#[derive(Clone, Debug)]
pub struct TakeBack {
    /// The session of the person.
    pub session_token: String,
    /// The grant.
    pub id: GrantId,
}

/// List the web addresses administrators allowed.
#[derive(Clone, Debug)]
pub struct WebAddressesQuery {
    /// The session of the administrator.
    pub session_token: String,
}

/// A web address apps may come back to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebAddressView {
    /// The address, as an app has to write it.
    pub address: String,
    /// When it was allowed.
    pub added_at: Timestamp,
}

/// The web addresses that were allowed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebAddresses {
    /// The addresses, by address.
    pub items: Vec<WebAddressView>,
}

impl Masked for WebAddresses {}

/// An administrator allows a web address.
#[derive(Clone, Debug)]
pub struct AddWebAddress {
    /// The session of the administrator.
    pub session_token: String,
    /// The return address.
    pub address: String,
}

/// An administrator takes a web address away.
#[derive(Clone, Debug)]
pub struct RemoveWebAddress {
    /// The session of the administrator.
    pub session_token: String,
    /// The return address.
    pub address: String,
}

/// Ask who a call to the MCP server is from, when a session of an app starts.
#[derive(Clone)]
pub struct McpIdentityQuery {
    /// The access token of an app, or an access key.
    pub token: String,
}

impl fmt::Debug for McpIdentityQuery {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("McpIdentityQuery(<hidden>)")
    }
}

/// Who an MCP session acts for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpIdentity {
    /// The person the session acts for, or answers for when an agent signed in.
    pub person: bb_core::id::UserId,
    /// The name the app gave itself; none for an access key.
    pub app: Option<String>,
    /// What the session is counted under: the grant of the app or the access key, so one app is one
    /// session however its calls come. Not a secret.
    pub credential: String,
    /// The agent, when an agent key signed in: its calls belong to the run of its attempt, not to
    /// a run of the session.
    pub agent: Option<bb_core::id::AgentId>,
}

/// Ask where the sign-in of AI apps is: for the two metadata documents. The address the request
/// came to is in the request info.
#[derive(Clone, Debug)]
pub struct OAuthMetadataQuery;

/// Where the sign-in of AI apps is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OAuthMetadata {
    /// The address of the server, without a slash at the end.
    pub origin: String,
    /// The address of the MCP server, the one resource a token is for.
    pub resource: String,
}

/// A session of an AI app begins: its run is opened.
#[derive(Clone)]
pub struct StartAppSession {
    /// The access token of the app, or an access key.
    pub token: String,
}

impl fmt::Debug for StartAppSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("StartAppSession(<hidden>)")
    }
}

/// The run of a session of an app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppSession {
    /// The run; none for an agent, whose calls belong to the run of its attempt.
    pub run: Option<bb_core::id::RunId>,
}

/// Sessions of apps end: their runs are closed.
#[derive(Clone, Debug)]
pub struct EndAppRuns {
    /// The runs.
    pub runs: Vec<bb_core::id::RunId>,
    /// How they ended.
    pub how: bb_core::run::RunEnd,
}
