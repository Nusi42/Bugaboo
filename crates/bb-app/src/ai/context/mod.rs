//! The context builder: what the assistant, an AI app or an agent reads (AI-04, AI-05, ADR-0013).
//!
//! `fence.rs` writes a text people wrote inside a fence that names who wrote it; `build.rs` turns
//! masked answers into the text the AI reads, takes out what is kept from AI and writes people as
//! M7-Q5 says. This file decides who wrote a text and how people are shown to a provider.

mod build;
mod fence;

use std::collections::HashMap;

use bb_core::id::{ProjectId, UserId};
use bb_policy::{Permission, Place};

pub use bb_core::ai::AuthorKind;
pub use build::{Part, PeopleAs, drop_withheld, people_in, render};
pub use fence::{READ_AS_DATA, fence, fenced_lines, only_inside_a_fence};

use crate::auth::{AppError, AuthStore, Ctx};

/// How people are told to the party that reads (M7-Q5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeopleMode {
    /// As the person who asks sees them: a local model, and AI apps, which act as the person.
    AsSeen,
    /// By role only: an outside provider before an administrator switched on *Send names of
    /// people*.
    RoleOnly,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What kind of author a person is in a project, as the AI is told. A person who sees all the
    /// issues is *someone in the project*, one who sees the project but only what they reported is
    /// a *Guest*, anybody else is *someone outside*.
    ///
    /// # Errors
    /// An internal error when the engine fails.
    pub(crate) async fn author_kind(
        &self,
        person: UserId,
        project: ProjectId,
    ) -> Result<AuthorKind, AppError> {
        let held = self.evaluate(person, Place::Project(project)).await?;
        Ok(if !held.sees_project() {
            AuthorKind::Outsider
        } else if held.allows(Permission::SeeIssues) {
            AuthorKind::Member
        } else {
            AuthorKind::Guest
        })
    }

    /// How the people in `parts` are written: as the person sees them, or by role.
    ///
    /// # Errors
    /// An internal error when the engine fails.
    pub(crate) async fn people_as(
        &self,
        you: UserId,
        mode: PeopleMode,
        parts: &[Part],
    ) -> Result<PeopleAs, AppError> {
        if mode == PeopleMode::AsSeen {
            return Ok(PeopleAs::AsSeen);
        }
        let mut roles = HashMap::new();
        for part in parts {
            let place = part.project.map_or(Place::Organization, Place::Project);
            for person in people_in(&part.value) {
                let held = self.evaluate(person, place).await?;
                if let Some(label) = held.first_role_name() {
                    roles.entry(person).or_insert_with(|| label.to_owned());
                }
            }
        }
        Ok(PeopleAs::RoleOnly { you, roles })
    }
}
