//! The large dataset (architecture §11, *Large dataset*, PERF-01): the sample's kinds of things at
//! the scale the budgets are measured on, written in big transactions.
//!
//! The first start, the fifty people, the organization and what the issues stand on (issue types,
//! own fields, labels, the workflow of Operations) are made by the same use cases as the sample.
//! The rest, hundreds of people and projects and every issue with its events, comments, tags and
//! files, is written in big transactions by [`crate::large_org`] and [`crate::large_write`], and the
//! history and the notifications are then read from the events by the code a server runs.

use bb_app::context::Clock as _;
use bb_core::time::SECONDS_PER_DAY;
use bb_dev_words::sample::{FieldPlan, IssuePlan, Layout};

use crate::error::{SampleError, Step};
use crate::generate::{Later, World};
use crate::issues_setup::{Setup, create_one_field, project_ids};
use crate::large_check::Facts;
use crate::large_org::Words;
use crate::organization::Organization;

/// The number of days in a month the history is counted in.
const MONTH_DAYS: i64 = 30;
/// How long before the dataset ends the last issue is made, in seconds.
const END_ROOM: i64 = SECONDS_PER_DAY;

/// The large dataset.
#[derive(Clone, Debug)]
pub struct Large {
    issues: u32,
    layout: Layout,
    facts: Facts,
    /// A run of `--check`: every event makes its notifications, and the check fails when it found
    /// nothing to look at in what Guests and Contributors are told.
    check: bool,
}

impl Large {
    /// The large dataset with `issues` issues, or the number of `tools/dev/sample.json`; `check`
    /// is a run of `--check`.
    ///
    /// # Errors
    /// [`SampleError::Layout`] when `tools/dev/sample.json` is not usable.
    pub fn new(issues: Option<u32>, check: bool) -> Result<Self, SampleError> {
        let layout = Layout::bundled()?;
        Ok(Self {
            issues: issues.unwrap_or(layout.large.issues),
            layout,
            facts: Facts::default(),
            check,
        })
    }

    /// What the check of the dataset needs to know; filled when the dataset is made.
    #[must_use]
    pub const fn facts(&self) -> &Facts {
        &self.facts
    }

    /// Whether this is a run of `--check`.
    #[must_use]
    pub const fn is_check(&self) -> bool {
        self.check
    }

    /// How many issues the dataset is made with.
    #[must_use]
    pub const fn issues(&self) -> u32 {
        self.issues
    }
}

impl Later for Large {
    fn history_seconds(&self) -> i64 {
        i64::from(self.layout.large.months) * MONTH_DAYS * SECONDS_PER_DAY + END_ROOM
    }

    async fn after_people(&mut self, world: &World<'_>) -> Result<(), SampleError> {
        Organization.after_people(world).await?;
        let names = self.layout.issues.clone();
        let mut plan = self.layout.large.clone();
        if self.check {
            plan.notified_percent = plan.check_notified_percent;
        }
        let token = world.administrator_session.to_owned();
        let setup = Setup::create(world, &token, &names).await?;
        let words = Words::read()?;
        let cost_center =
            cost_center(world, &token, &words.fields.cost_center, &names, &setup).await?;
        let begin = world.clock.now().as_second() + 3600;
        let end = world.start.as_second() - END_ROOM;
        let org = crate::large_org::grow(world, &plan, &self.layout, &words).await?;
        let (guests, contributors) = Facts::people(&org.folk);
        self.facts = Facts {
            guests,
            contributors,
            cost_center: Some(cost_center),
        };
        let bank = crate::large_state::read(
            world,
            &setup,
            cost_center,
            &org,
            (&plan, &names),
            (world.clock.now(), &words),
        )
        .await?;
        crate::large_write::run(
            world,
            &bank,
            &org,
            crate::large_write::Span {
                issues: self.issues,
                begin,
                end,
            },
        )
        .await
    }
}

/// Makes the own field *Cost code* for every issue and hides it from Contributors, who may not
/// see what it is charged to.
async fn cost_center(
    world: &World<'_>,
    token: &str,
    name: &str,
    names: &IssuePlan,
    setup: &Setup,
) -> Result<bb_core::id::FieldId, SampleError> {
    world.tick();
    let projects = project_ids(world).await?;
    let plan = FieldPlan {
        name: name.to_owned(),
        kind: "text".to_owned(),
        issue_types: Vec::new(),
        projects: Vec::new(),
        choices: Vec::new(),
    };
    let field = create_one_field(world, token, &plan, names, &setup.types, &projects).await?;
    world
        .hide_from_contributors(token, field.id, Step::Levels)
        .await?;
    Ok(field.id)
}
