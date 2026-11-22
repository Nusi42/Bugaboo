//! People who left, a finished export and backups (M9 part 8, PRIV-03, PRIV-04, OPS-07), made
//! through the use cases the screens call, as the last work of the sample.
//!
//! An administrator switches one Team member off, and switches off and removes another, who handed
//! their work over in a comment on their last work day: their comments stay as *Former user*, and
//! their work days stay under the legal-hold name, which *HR* still sees. The person the plan names
//! asks for their own data and the job makes the file. Then the administrator makes the backups.
//! Who leaves is never a person the demos name, the holder of the lock or the writer of the draft
//! that the sample leaves behind.

use bb_app::access::StartingRole;
use bb_app::auth::AuthApi;
use bb_app::backups::CreateBackup;
use bb_app::comments::{AddComment, add_comment};
use bb_app::people::SwitchOff;
use bb_app::privacy::export::AskMyData;
use bb_app::privacy::removal::RemovePerson;
use bb_app::working_hours::{
    StartWorkDay, Tap, TapWorkDay, WorkingHoursStore as _, start_work_day, tap_work_day,
};
use bb_core::id::UserId;
use bb_dev_words::sample::PrivacyPlan;
use jiff::civil::Date;

use crate::error::{SampleError, Step};
use crate::issues::Maker;

/// The people who left the organization in the sample.
#[derive(Clone, Copy, Debug)]
pub struct Gone {
    /// Switched off: they may not sign in, and everything they did stays theirs.
    pub switched_off: UserId,
    /// Removed: shown as *Former user*, with their work days under the legal-hold name.
    pub removed: UserId,
}

impl Gone {
    /// Whether the person left: they cannot sign in any more.
    #[must_use]
    pub fn left(&self, person: UserId) -> bool {
        person == self.switched_off || person == self.removed
    }
}

impl Maker<'_, '_> {
    /// The people who leave: the last two Team members of the plan of the people who no demo
    /// names, who have no access key and no code app, and who did not leave the lock or the draft
    /// behind. The last of them is removed, the one before switched off.
    fn leavers(&self) -> Result<(usize, usize), SampleError> {
        let left_behind = self.hands.workers.get(..2).unwrap_or_default();
        let mut candidates = self
            .world
            .people
            .iter()
            .enumerate()
            .filter(|(at, made)| {
                made.person.role == StartingRole::TeamMember
                    && !made.person.named_in_demos()
                    && made.person.access_key.is_none()
                    && !made.person.code_app
                    && !left_behind.contains(at)
            })
            .map(|(at, _)| at);
        let removed = candidates.next_back().ok_or(SampleError::PersonMissing)?;
        let switched_off = candidates.next_back().ok_or(SampleError::PersonMissing)?;
        Ok((switched_off, removed))
    }

    /// The last day of the person who is removed: they start it, hand their work over in a comment
    /// on the first issue of the plan they may comment on, and end it.
    async fn last_day(&mut self, person: usize, text: &str) -> Result<(), SampleError> {
        let token = self.hands.token(person).await?;
        let ctx = self.world.services.ctx(self.world.request());
        self.world.tick();
        start_work_day(
            &ctx,
            StartWorkDay {
                session_token: token.clone(),
                device_zone: None,
            },
        )
        .await
        .map_err(|error| SampleError::step(Step::Privacy, &error))?;
        // The version of the day, as the person's screen has it: their newest day.
        let version = self
            .world
            .storage
            .work_day_before(self.world.people[person].id, Date::MAX)
            .await?
            .map_or(1, |stored| stored.day.version);
        let issues: Vec<String> = self
            .issues
            .iter()
            .flatten()
            .map(|made| made.id.to_string())
            .collect();
        self.world.tick();
        let mut written = false;
        for issue in issues {
            let comment = AddComment {
                session_token: token.clone(),
                issue,
                text: text.to_owned(),
            };
            if add_comment(&ctx, comment).await.is_ok() {
                written = true;
                break;
            }
        }
        if !written {
            return Err(SampleError::NotMade(text.to_owned()));
        }
        self.world.tick();
        tap_work_day(
            &ctx,
            TapWorkDay {
                session_token: token,
                version,
                tap: Tap::End,
            },
        )
        .await
        .map_err(|error| SampleError::step(Step::Privacy, &error))?;
        Ok(())
    }

    /// Part 8: the people who leave, the export of the person the plan names, and the backups.
    pub(crate) async fn let_people_go(&mut self, plan: &PrivacyPlan) -> Result<Gone, SampleError> {
        let world = self.world;
        let api: &dyn AuthApi = world.services;
        let failed = |step| move |error| SampleError::step(step, &error);
        let (switched_off, removed) = self.leavers()?;
        self.last_day(removed, &plan.hand_over).await?;
        let gone = Gone {
            switched_off: world.people[switched_off].id,
            removed: world.people[removed].id,
        };
        let administrator_at = world.administrator()?;
        let administrator = world.session_of(administrator_at).await?;
        for person in [gone.switched_off, gone.removed] {
            world.tick();
            api.switch_off(
                world.request(),
                SwitchOff {
                    session_token: administrator.clone(),
                    person,
                },
            )
            .await
            .map_err(failed(Step::Privacy))?;
        }
        world.tick();
        api.remove_person(
            world.request(),
            RemovePerson {
                session_token: administrator.clone(),
                person: gone.removed,
                comments: None,
                password: world.people[administrator_at].password.clone(),
            },
        )
        .await
        .map_err(failed(Step::Privacy))?;
        let asking = world
            .people
            .iter()
            .position(|made| made.person.name == plan.export_of)
            .ok_or_else(|| SampleError::NotMade(plan.export_of.clone()))?;
        let token = world.session_of(asking).await?;
        world.tick();
        api.ask_for_my_data(
            world.request(),
            AskMyData {
                session_token: token,
            },
        )
        .await
        .map_err(failed(Step::Privacy))?;
        // The job that finishes the removal and the job that writes the file, as a server runs them.
        world.run_the_jobs().await?;
        for _ in 0..plan.backups {
            world.tick();
            api.create_backup(
                world.request(),
                CreateBackup {
                    session_token: administrator.clone(),
                },
            )
            .await
            .map_err(failed(Step::Backups))?;
        }
        Ok(gone)
    }
}
