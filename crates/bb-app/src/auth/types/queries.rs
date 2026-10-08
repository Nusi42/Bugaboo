//! What the sign-in use cases take when they only read: every one ends in `Query` (ARCH-EVENTS).

use bb_core::id::RoleId;

// ---- Inputs that only read ----

/// What the app needs to know before anybody signs in.
pub struct StatusQuery;

/// Who asks, judged before anything else of the request is read: a session passes, an access key
/// is turned away from the routes of the person's own sign-in data.
pub struct RequireSessionQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// Whether the person behind a session or an access key may see how the server is doing.
pub struct SeeServerStatusQuery {
    /// The secret of the session, or the access key, of the request.
    pub session_token: String,
}

/// Who is behind this session.
pub struct AuthenticateQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// The sessions of the person.
pub struct SessionsQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// How the second step is set up.
pub struct TwoStepQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// A challenge for a passkey as the second step of signing in.
pub struct PasskeySecondStepQuery {
    /// The secret of the pending session.
    pub session_token: String,
}

/// A challenge for signing in with a passkey alone.
pub struct PasskeySignInQuery;

/// A challenge for adding a passkey. The password is asked again, so a stolen session cannot
/// add one.
pub struct PasskeyRegistrationQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The password, asked again for this.
    pub password: String,
}

/// The access keys of the person, one page at a time.
pub struct AccessKeysQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Where the page after the last one starts (`next_cursor` of the page before), if any.
    pub cursor: Option<String>,
    /// How many keys a page holds at most; more than `bb_core::paging::PAGE_SIZE_MAX` is cut to
    /// it.
    pub limit: u32,
}

/// The projects a person may choose for an access key, one page at a time.
pub struct ProjectChoicesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// Where the page after the last one starts (`next_cursor` of the page before), if any.
    pub cursor: Option<String>,
    /// How many projects a page holds at most.
    pub limit: u32,
}

/// How long a new access key lives in this organization.
pub struct AccessKeyRulesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// How many people with a role have no second step yet.
pub struct SecondStepGapQuery {
    /// The secret of the session of the request (somebody who manages roles).
    pub session_token: String,
    /// The role.
    pub role: RoleId,
}

/// The open invitations, one page at a time.
pub struct InvitationsQuery {
    /// The secret of the session of the request (an administrator).
    pub session_token: String,
    /// Where the page after the last one starts (`next_cursor` of the page before), if any.
    pub cursor: Option<String>,
    /// How many invitations a page holds at most; more than `bb_core::paging::PAGE_SIZE_MAX` is
    /// cut to it.
    pub limit: u32,
}

/// Which address an invitation is for.
pub struct InvitationLookupQuery {
    /// The secret from the link.
    pub secret: String,
}

/// Who may sign up on their own.
pub struct SignUpPolicyQuery {
    /// The secret of the session of the request (an administrator).
    pub session_token: String,
}
