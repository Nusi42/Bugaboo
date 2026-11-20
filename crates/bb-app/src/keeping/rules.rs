//! The rules of *Keep and delete rules* (PRIV-05, ADR-0045, decision 2): every row of `KEEP_KINDS`
//! with its group, the period in force with the organization's settings, the setting that changes
//! it and its floor. Every person who signs in may read them, because they have a right to know how
//! long their data is kept (*My settings › My data*, GDPR Art. 15(1)(d)); only an administrator
//! changes them, through the settings routes that exist. This only reads.

use bb_core::keeping::{KEEP_KINDS, KeepGroup, KeepKind, Length};
use bb_policy::{Authorized, Masked, Place};

use crate::auth::{AppError, AuthStore, Ctx};

/// The rules, for the page *Keep and delete rules*.
pub struct KeepRulesQuery {
    /// The secret of the session of the request.
    pub session_token: String,
}

/// One row of the rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeepRuleView {
    /// The kind of data.
    pub kind: KeepKind,
    /// Who deletes it after its period.
    pub group: KeepGroup,
    /// The period in force.
    pub length: Length,
    /// Whether a record goes when the period is over; not for backups, which the rotation keeps
    /// whatever their age.
    pub fixed_end: bool,
    /// The key of the setting that changes the period; none for a fixed period.
    pub setting: Option<&'static str>,
    /// The shortest period the setting allows; none for a fixed period.
    pub floor: Option<i64>,
}

/// Every row of the rules, in the order of the page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeepRules {
    /// The rows.
    pub rules: Vec<KeepRuleView>,
}

impl Masked for KeepRules {}

/// The rules with the periods in force.
///
/// # Errors
/// `auth.not-signed-in` without a session; an access key is refused, as for the person's own data.
pub async fn keep_rules<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: KeepRulesQuery,
) -> Result<Authorized<KeepRules>, AppError> {
    let actor = ctx.session_actor(&input.session_token).await?;
    let evaluation = ctx.evaluation_of(&actor, Place::Organization).await?;
    let layers = ctx.setting_layers_of(None).await?;
    let rules = KEEP_KINDS
        .iter()
        .map(|row| KeepRuleView {
            kind: row.kind,
            group: row.group,
            length: row.length(&layers),
            fixed_end: row.fixed_end(),
            setting: row.setting().map(|setting| setting.definition().key()),
            floor: row.floor(),
        })
        .collect();
    Ok(evaluation.authorize(KeepRules { rules }))
}
