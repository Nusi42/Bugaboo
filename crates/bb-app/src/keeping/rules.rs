//! The rules of *Keep and delete rules* as an administrator reads them (PRIV-05, ADR-0045,
//! decision 2): every row of `KEEP_KINDS` with its group, the period in force with the
//! organization's settings, the setting that changes it and its floor. The periods change through
//! the settings routes that exist; this only reads.

use bb_core::keeping::{KEEP_KINDS, KeepGroup, KeepKind, Length};
use bb_policy::{Authorized, Masked};

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
/// `access.forbidden` without *Change organization settings*.
pub async fn keep_rules<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: KeepRulesQuery,
) -> Result<Authorized<KeepRules>, AppError> {
    let (_, evaluation) = ctx.keeper(&input.session_token).await?;
    let layers = ctx.setting_layers_of(None).await?;
    let rules = KEEP_KINDS
        .iter()
        .map(|row| KeepRuleView {
            kind: row.kind,
            group: row.group,
            length: row.length(&layers),
            setting: row.setting().map(|setting| setting.definition().key()),
            floor: row.floor(),
        })
        .collect();
    Ok(evaluation.authorize(KeepRules { rules }))
}
