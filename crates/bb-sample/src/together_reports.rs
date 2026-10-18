//! Contributors and Guests report issues and comment under them; Contributors change what they reported.

use bb_app::comments::{AddComment, add_comment};
use bb_app::issues::{ChangeIssue, CreateIssue, IssueQuery, change_issue, create_issue, issue};
use bb_app::tags::{AddTag, add_tag};
use bb_core::id::IssueId;
use bb_dev_words::sample::Layout;

use crate::error::{SampleError, Step};
use crate::issues::{MadeIssue, Maker};
use crate::issues_setup::{id_in, json};
use crate::issues_text::describe;
use crate::organization::projects_of;
use crate::people_plan::SamplePerson;
use crate::together::{COMMENTED_PERCENT, Reported, Work};

impl Maker<'_, '_> {
    /// Contributors and Guests report issues in the projects they have a role on, and most of them
    /// write a comment under their own report.
    pub(crate) async fn people_report(
        &mut self,
        work: &mut Work,
        people: &[SamplePerson],
    ) -> Result<(), SampleError> {
        let organization = crate::organization::plan(people, &Layout::bundled()?.organization);
        for (who, total) in [
            (work.contributors.clone(), work.counts.contributor_reports),
            (work.guests.clone(), work.counts.guest_reports),
        ] {
            let reporters: Vec<(usize, Vec<String>)> = who
                .iter()
                .map(|person| (*person, projects_of(&organization, *person)))
                .filter(|(_, projects)| !projects.is_empty())
                .collect();
            if reporters.is_empty() {
                return Err(SampleError::NotMade("reporters".to_owned()));
            }
            for number in 0..usize::try_from(total).unwrap_or(0) {
                let (person, projects) = &reporters[number % reporters.len()];
                let project = &projects[(number / reporters.len()) % projects.len()];
                self.report(work, *person, project).await?;
            }
        }
        Ok(())
    }

    /// One report: a bug in a place of the project, and sometimes a comment under it.
    pub(crate) async fn report(
        &mut self,
        work: &mut Work,
        person: usize,
        project: &str,
    ) -> Result<(), SampleError> {
        let family = self
            .names
            .projects
            .iter()
            .find(|each| each.short_name == project)
            .ok_or_else(|| SampleError::NotMade(project.to_owned()))?;
        let place = work
            .random
            .pick(&family.places)
            .map_or(project, |place| place.short_name.as_str())
            .to_owned();
        let template = work
            .random
            .pick(&work.words.bug.titles)
            .cloned()
            .unwrap_or_default();
        let title = work.words.fill(&template, &family.area, &mut work.random);
        let description = describe(
            &work.words,
            &work.words.bug,
            true,
            &family.area,
            &mut work.random,
        );
        let token = self.hands.token(person).await?;
        self.world.tick();
        let ctx = self.world.services.ctx(self.world.request());
        let made = json(
            create_issue(
                &ctx,
                CreateIssue {
                    session_token: token.clone(),
                    project: place,
                    issue_type: *self
                        .setup
                        .types
                        .first()
                        .ok_or_else(|| SampleError::NotMade("issue type".to_owned()))?,
                    title,
                    description: Some(description),
                    status: None,
                    priority: None,
                    assigned_to: None,
                    fields: std::collections::BTreeMap::new(),
                },
            )
            .await,
            Step::Issue,
        )?;
        let id: IssueId = id_in(&made, "id")?;
        work.reported.push(Reported { id, author: person });
        if work.random.chance(COMMENTED_PERCENT) {
            let text = work
                .random
                .pick(&work.words.comments.first)
                .cloned()
                .unwrap_or_default();
            self.world.tick();
            json(
                add_comment(
                    &ctx,
                    AddComment {
                        session_token: token,
                        issue: id.to_string(),
                        text: work.words.fill(&text, "", &mut work.random),
                    },
                )
                .await,
                Step::Comment,
            )?;
        }
        Ok(())
    }

    /// Contributors change what they reported: a Contributor may change the title and the tags of
    /// their own issues, not the status.
    pub(crate) async fn contributors_change(&mut self, work: &mut Work) -> Result<(), SampleError> {
        let wanted = usize::try_from(work.counts.contributor_changes).unwrap_or(usize::MAX);
        let mut done = 0_usize;
        let mine: Vec<Reported> = work
            .reported
            .iter()
            .filter(|each| work.contributors.contains(&each.author))
            .cloned()
            .collect();
        if mine.is_empty() {
            return Err(SampleError::NotMade("issues of Contributors".to_owned()));
        }
        while done < wanted {
            let own = mine[done % mine.len()].clone();
            let token = self.hands.token(own.author).await?;
            let ctx = self.world.services.ctx(self.world.request());
            self.world.tick();
            if done.is_multiple_of(2) {
                self.retitle(work, &ctx, &token, own.id).await?;
            } else {
                let tag = work
                    .random
                    .pick(&self.names.tags)
                    .cloned()
                    .unwrap_or_default();
                add_tag(
                    &ctx,
                    AddTag {
                        session_token: token,
                        issue: own.id.to_string(),
                        name: tag,
                    },
                )
                .await
                .map_err(|error| SampleError::step(Step::Tags, &error))?;
            }
            done += 1;
        }
        Ok(())
    }

    /// A new title for an issue, made by the person whose session it is.
    pub(crate) async fn retitle(
        &mut self,
        work: &mut Work,
        ctx: &bb_app::auth::Ctx<'_, bb_store::AuthStorage>,
        token: &str,
        issue_id: IssueId,
    ) -> Result<(), SampleError> {
        let detail = json(
            issue(
                ctx,
                IssueQuery {
                    session_token: token.to_owned(),
                    issue: issue_id.to_string(),
                },
            )
            .await,
            Step::Change,
        )?;
        let version = MadeIssue::of(&detail["issue"])?.version;
        let template = work
            .random
            .pick(&work.words.bug.titles)
            .cloned()
            .unwrap_or_default();
        let title = work.words.fill(&template, "app", &mut work.random);
        json(
            change_issue(
                ctx,
                ChangeIssue {
                    session_token: token.to_owned(),
                    issue: issue_id.to_string(),
                    version,
                    issue_type: None,
                    title: Some(title),
                    status: None,
                    priority: None,
                    assigned_to: None,
                    fields: std::collections::BTreeMap::new(),
                },
            )
            .await,
            Step::Change,
        )?;
        Ok(())
    }
}
