//! Whose work days a request is about, and what the reader may do with them: the one rule of who
//! sees and who changes another person's working hours (ADR-0035, decision 8).
//!
//! - One's own days, always.
//! - Another person's only with *See working hours of others* (to read) or *Change working hours of
//!   others* (to change), both for the whole organization. Without either, another person's days
//!   answer exactly like a person that does not exist, so nobody learns who has days.
//! - *Totals only* shows week totals and nothing else, for every reader of another person's days,
//!   and turns changes of them off.

use std::sync::Arc;

use bb_core::ErrorCode;
use bb_core::id::UserId;
use bb_core::people_kinds::PeopleKind;
use bb_core::settings::Layers;
use bb_core::working_time::totals_only;
use bb_policy::{Evaluation, PeopleContext, Permission, PersonRef, Place};

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx};

/// The days of one person, as one reader may use them.
pub(super) struct Whose {
    /// The person whose days they are.
    pub(super) person: UserId,
    /// Whether they are the reader's own.
    pub(super) own: bool,
    /// What the reader may do for the whole organization.
    pub(super) evaluation: Arc<Evaluation>,
    /// The organization's settings.
    pub(super) layers: Layers,
}

impl Whose {
    /// Whether *Totals only* hides the days from this reader: never their own.
    pub(super) fn totals_only(&self) -> bool {
        !self.own && totals_only(&self.layers)
    }

    /// Whether the reader may change the days.
    pub(super) fn may_change(&self) -> bool {
        self.own
            || (self
                .evaluation
                .allows(Permission::ChangeWorkingHoursOfOthers)
                && !self.totals_only())
    }

    /// How the people of these days are shown to the reader: whoever changed one's own days is
    /// named to the person; on *Working hours of others* the holder of the permission sees names.
    pub(super) const fn context(&self) -> PeopleContext {
        if self.own {
            PeopleContext::OwnWorkDay
        } else {
            PeopleContext::WorkingHours
        }
    }
}

impl<S: AuthStore> Ctx<'_, S> {
    /// Who asks about the days of `person` (their own when `None`): one's own working hours are a
    /// record of the person in a browser, which no access key reaches; another person's are judged
    /// by the permissions of working hours, which no key holds either (ADR-0025, amendment M6).
    ///
    /// # Errors
    /// The refusals of signing in, and `auth.access-key-not-allowed` for a key and one's own days.
    pub(super) async fn asking(
        &self,
        session_token: &str,
        person: Option<UserId>,
    ) -> Result<Actor, AppError> {
        match person {
            None => self.session_actor(session_token).await,
            Some(_) => self.actor(session_token).await,
        }
    }

    /// The days of `person` (the actor's own when `None`) as the actor may read them.
    ///
    /// # Errors
    /// `resource.not-found` for another person without *See working hours of others*, and for a
    /// person that does not exist.
    pub(super) async fn days_to_read(
        &self,
        actor: &Actor,
        person: Option<UserId>,
    ) -> Result<Whose, AppError> {
        let whose = self.whose(actor, person).await?;
        if whose.own || whose.evaluation.allows(Permission::SeeWorkingHoursOfOthers) {
            self.known_person(&whose).await?;
            return Ok(whose);
        }
        Err(ErrorCode::NotFound.into())
    }

    /// The days of `person` (the actor's own when `None`) as the actor may change them.
    ///
    /// # Errors
    /// `resource.not-found` for another person whom the actor may neither see nor change and for a
    /// person that does not exist, `access.forbidden` without *Change working hours of others*, and
    /// `working-hours.week-totals-only` while *Totals only* is on.
    pub(super) async fn days_to_change(
        &self,
        actor: &Actor,
        person: Option<UserId>,
    ) -> Result<Whose, AppError> {
        let whose = self.whose(actor, person).await?;
        if whose.own {
            return Ok(whose);
        }
        let sees = whose.evaluation.allows(Permission::SeeWorkingHoursOfOthers);
        let changes = whose
            .evaluation
            .allows(Permission::ChangeWorkingHoursOfOthers);
        if !sees && !changes {
            return Err(ErrorCode::NotFound.into());
        }
        self.known_person(&whose).await?;
        if !changes {
            return Err(ErrorCode::Forbidden.into());
        }
        if whose.totals_only() {
            return Err(ErrorCode::TotalsOnly.into());
        }
        Ok(whose)
    }

    /// The person of the days as the reader sees them.
    pub(super) async fn person_of(&self, whose: &Whose) -> Result<PersonRef, AppError> {
        self.person_ref(
            &whose.evaluation,
            PeopleKind::TimeEntries,
            whose.context(),
            whose.person,
        )
        .await
    }

    async fn whose(&self, actor: &Actor, person: Option<UserId>) -> Result<Whose, AppError> {
        let person = person.unwrap_or(actor.person.id);
        Ok(Whose {
            person,
            // A key never reaches its owner's own days: it is judged by the permissions of working
            // hours like anybody else's, and no key holds them.
            own: person == actor.person.id && actor.key.is_none(),
            evaluation: self.evaluation_of(actor, Place::Organization).await?,
            layers: self.setting_layers_of(None).await?,
        })
    }

    /// Fails like a missing person when the person does not exist.
    async fn known_person(&self, whose: &Whose) -> Result<(), AppError> {
        if whose.own || self.person_known(whose.person).await?.is_some() {
            Ok(())
        } else {
            Err(ErrorCode::NotFound.into())
        }
    }
}
