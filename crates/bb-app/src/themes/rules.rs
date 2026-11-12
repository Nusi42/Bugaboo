//! Who may see and change an own theme, and the checks of what a save would store: the values an
//! own theme may set, the built-in theme it extends, its name and how readable it is (THM-03,
//! THM-04, ADR-0026 update M8). Written once, for every use case of themes and for the edit lock.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::contrast::Rating;
use bb_core::edit::{EditWhat, THEME_PART};
use bb_core::event::RecordKind;
use bb_core::id::{ThemeId, UserId};
use bb_core::technical::TechnicalName;
use bb_core::theme::{
    BuiltIn, OwnChoice, OwnTheme, OwnValues, ThemeKind, ThemeName, built_in_theme, readability,
};
use bb_policy::{Evaluation, Permission, Place};
use serde::Serialize;

use super::types::{ChosenTheme, ThemeView};
use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, entity, internal};

/// A pair of colors that is *Hard to read*, by the tokens it is made of.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NamedPair {
    /// The first color: the text, the line or the state.
    pub first: String,
    /// The second color: the background, the surface or the other state.
    pub second: String,
    /// The color to change to mend it.
    pub place: String,
    /// Whether the pair is read with the colors for more contrast.
    pub more_contrast: bool,
}

/// What a choice of an own theme counts as.
pub(crate) enum ChoiceInForce {
    /// The theme, which the person may use.
    Own(ChosenTheme),
    /// The built-in theme it extended: the theme is gone, or the person may not use it.
    InsteadOf(&'static str),
}

/// What a person wants to do with a theme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Need {
    /// Use it: read it, its style sheet, download it, copy it.
    Use,
    /// Change it or take it away.
    Change,
}

/// Whether `person` may use `theme`: a saved theme for everyone, or one of their own (also one
/// that was never saved). Everybody else is answered as if it did not exist.
pub(crate) fn usable_by(theme: &OwnTheme, person: UserId) -> bool {
    theme.maker == person || (theme.kind == ThemeKind::ForEveryone && theme.saved_at.is_some())
}

/// Whether `person`, who holds `held` in the organization, may know `theme`: one they may use, or
/// a theme for everyone that was never saved when they look after themes (*Change organization
/// settings*), so a theme somebody left unsaved can be finished or taken away.
pub(crate) fn known_to(theme: &OwnTheme, person: UserId, held: &Evaluation) -> bool {
    usable_by(theme, person) || looks_after_unsaved(theme, held)
}

/// Whether `held` lets its holder see the themes for everyone that were never saved.
pub(crate) fn sees_unsaved_for_everyone(held: &Evaluation) -> bool {
    held.allows(Permission::ChangeOrganizationSettings)
}

fn looks_after_unsaved(theme: &OwnTheme, held: &Evaluation) -> bool {
    theme.kind == ThemeKind::ForEveryone && sees_unsaved_for_everyone(held)
}

/// The thing a theme's edit lock and draft are about.
pub(crate) fn editing_what(theme: ThemeId) -> Result<EditWhat, AppError> {
    Ok(EditWhat {
        record: entity(RecordKind::Theme, theme),
        part: TechnicalName::new(THEME_PART)
            .map_err(|error| AppError::Internal(error.to_string()))?,
    })
}

/// The built-in theme an own theme extends.
pub(crate) fn built_in_of(id: &str) -> Result<&'static BuiltIn, AppError> {
    built_in_theme(id).ok_or(AppError::Refused(ErrorCode::InvalidRequest))
}

/// The name of a theme, as a person wrote it.
pub(crate) fn name_of(text: &str) -> Result<ThemeName, AppError> {
    ThemeName::new(text).map_err(|_| AppError::Refused(ErrorCode::InvalidRequest))
}

/// The values of a theme from JSON in the shape of the export; each value that may not be set is
/// named in the refusal.
pub(crate) fn values_of(json: &serde_json::Value) -> Result<OwnValues, AppError> {
    OwnValues::from_json(json).map_err(AppError::ThemeValuesRefused)
}

/// The pairs of the theme that extends `extends` with `values` that are *Hard to read*, each once.
pub(crate) fn hard_pairs(extends: &str, values: &OwnValues) -> Result<Vec<NamedPair>, AppError> {
    // `None` only for a built-in theme that does not exist, which the theme was checked against.
    let rated = readability(extends, values).ok_or(AppError::Refused(ErrorCode::InvalidRequest))?;
    let mut pairs: Vec<NamedPair> = Vec::new();
    for found in rated
        .into_iter()
        .filter(|pair| pair.rating == Rating::HardToRead)
    {
        let pair = NamedPair {
            first: found.first,
            second: found.second,
            place: found.place,
            more_contrast: found.more_contrast,
        };
        if !pairs.contains(&pair) {
            pairs.push(pair);
        }
    }
    Ok(pairs)
}

/// Refuses a theme for everyone with pairs that are *Hard to read*; a personal theme is saved with
/// them, and they are its warning.
pub(crate) fn refuse_hard_for_everyone(
    kind: ThemeKind,
    pairs: Vec<NamedPair>,
) -> Result<Vec<NamedPair>, AppError> {
    if kind == ThemeKind::ForEveryone && !pairs.is_empty() {
        return Err(AppError::HardToRead(pairs));
    }
    Ok(pairs)
}

/// The view of a theme for a person who may use it.
pub(crate) fn view_of(theme: OwnTheme, held: &Evaluation, person: UserId) -> ThemeView {
    let may_change = match theme.kind {
        ThemeKind::Personal => theme.maker == person,
        ThemeKind::ForEveryone => held.allows(Permission::ChangeOrganizationSettings),
    };
    let light = built_in_theme(theme.extends.as_str()).is_some_and(|base| base.light);
    ThemeView {
        choice: OwnChoice::value(theme.extends.as_str(), theme.id),
        may_change,
        light,
        theme,
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What the actor needs to make or change a theme of `kind`: *Change organization settings*
    /// for a theme for everyone, nothing more than being signed in for a personal one.
    ///
    /// # Errors
    /// `access.forbidden` for a theme for everyone without the permission.
    pub(crate) async fn may_create_theme(
        &self,
        actor: &Actor,
        kind: ThemeKind,
    ) -> Result<Arc<Evaluation>, AppError> {
        match kind {
            ThemeKind::ForEveryone => {
                self.require(
                    actor,
                    Permission::ChangeOrganizationSettings,
                    Place::Organization,
                )
                .await
            }
            ThemeKind::Personal => self.evaluation_of(actor, Place::Organization).await,
        }
    }

    /// The theme with what the actor holds, when they may do what they need with it.
    ///
    /// # Errors
    /// `resource.not-found` for a theme that is not there or that the actor may not use;
    /// `access.forbidden` for a theme for everyone they may use but not change.
    pub(crate) async fn theme_for(
        &self,
        actor: &Actor,
        theme: ThemeId,
        need: Need,
    ) -> Result<(OwnTheme, Arc<Evaluation>), AppError> {
        let in_organization = self.evaluation_of(actor, Place::Organization).await?;
        let found = self
            .store()
            .own_theme(theme)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|found| known_to(found, actor.person.id, &in_organization))
            .ok_or(AppError::Refused(ErrorCode::NotFound))?;
        let held = match need {
            Need::Use => in_organization,
            Need::Change => self.may_create_theme(actor, found.kind).await?,
        };
        Ok((found, held))
    }

    /// What a choice of a theme counts as for `person` (`None` for the value of the organization or
    /// a project): `None` for a built-in theme, the own theme when they may use it, else the
    /// built-in theme it extended (ADR-0026, update M8).
    ///
    /// # Errors
    /// The store failed.
    pub(crate) async fn theme_choice_in_force(
        &self,
        person: Option<UserId>,
        text: &str,
    ) -> Result<Option<ChoiceInForce>, AppError> {
        let Some(own) = OwnChoice::read(text) else {
            return Ok(None);
        };
        let found = self
            .store()
            .own_theme(own.theme)
            .await
            .map_err(|error| internal::<S>(&error))?
            .filter(|theme| {
                theme.saved_at.is_some()
                    && theme.extends.as_str() == own.extends.id
                    && match person {
                        Some(person) => usable_by(theme, person),
                        None => theme.kind == ThemeKind::ForEveryone,
                    }
            });
        Ok(Some(match found {
            Some(theme) => ChoiceInForce::Own(ChosenTheme {
                id: theme.id,
                extends: own.extends.id,
                version: theme.version,
            }),
            None => ChoiceInForce::InsteadOf(own.extends.id),
        }))
    }
}
