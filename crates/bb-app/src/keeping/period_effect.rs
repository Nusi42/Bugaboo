//! What a changed keep period would do, before it is saved (ADR-0045, decision 2): the page of the
//! rules says "This deletes 312 notifications within a day" first. For a kind Bugaboo deletes by
//! itself it is how many records its job would delete with the changed period; for a kind an
//! administrator deletes, how many would be past the changed period, which nothing deletes until an
//! administrator says so. Both are counted with the one cutoff of the kind, from the settings as
//! they would be ([`bb_core::keeping::KeepRule::with_period`]); nothing is changed.

use bb_core::ErrorCode;
use bb_core::keeping::{KeepGroup, KeepKind, rule};
use bb_policy::{Authorized, Masked};

use crate::auth::{AppError, AuthStore, Ctx, internal};

/// What a keep period of `count` (in the unit of its setting) would do to a kind.
pub struct PeriodEffectQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The kind of data.
    pub kind: KeepKind,
    /// The period as it would be saved, in the unit of the setting of the kind.
    pub count: i64,
}

/// How many records a changed keep period touches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeriodEffect {
    /// The kind of data.
    pub kind: KeepKind,
    /// Who deletes them: Bugaboo within a day, or an administrator when they say so.
    pub group: KeepGroup,
    /// How many records are past the changed period.
    pub records: u64,
}

impl Masked for PeriodEffect {}

/// How many records of a kind a changed keep period would delete, or leave past their period.
///
/// # Errors
/// `access.forbidden` without *Change organization settings*, `resource.not-found` for a kind whose
/// period is not one setting (a fixed period, the rotation of backups), `request.invalid` for a
/// period the setting refuses (below its floor, above its most).
pub async fn period_effect<S: AuthStore>(
    ctx: &Ctx<'_, S>,
    input: PeriodEffectQuery,
) -> Result<Authorized<PeriodEffect>, AppError> {
    let (_, evaluation) = ctx.keeper(&input.session_token).await?;
    let row = rule(input.kind);
    if row.setting().is_none() || !row.fixed_end() {
        return Err(ErrorCode::NotFound.into());
    }
    let layers = ctx.setting_layers_of(None).await?;
    let changed = row
        .with_period(&layers, input.count)
        .ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    let records = match row.group {
        KeepGroup::ByItself => {
            let before = ctx.keep_cutoff_with(input.kind, &changed)?;
            ctx.store()
                .count_past(input.kind, before)
                .await
                .map_err(|error| internal::<S>(&error))?
        }
        KeepGroup::ByAnAdministrator => {
            let before = ctx.kept_from_today_with(row, &changed)?;
            ctx.store()
                .old_records(input.kind, before)
                .await
                .map_err(|error| internal::<S>(&error))?
                .iter()
                .map(|old| old.count)
                .sum()
        }
    };
    Ok(evaluation.authorize(PeriodEffect {
        kind: input.kind,
        group: row.group,
        records,
    }))
}
