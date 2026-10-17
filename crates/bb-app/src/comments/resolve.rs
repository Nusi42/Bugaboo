//! Turning the tokens of a text into things (ISS-05, M3-Q7, ADR-0011): which `@name` names whom,
//! which issue ID is an issue the reader may see, and who a comment puts into *Mentions*.
//!
//! `bb-markdown` only says where the tokens are. This is the server's decision, made for one
//! viewer, in one place:
//! - An issue ID is a chip only when the reader may see the issue; a hidden issue stays text.
//! - A mention is a chip only when it names exactly one thing the reader may name. What the reader
//!   may name follows the kind of people information *Mentions* (M3-Q7): by *Names* people and
//!   groups; by *Role only* groups and roles, never a person by name; by *Hidden* nothing.
//! - A group puts the comment into each member's *Mentions*, a role into those who have it where
//!   the issue is (a cap, `MENTION_RECIPIENTS_MAX`, keeps a big group from filling the table).

use std::sync::{Arc, PoisonError};

use bb_core::access::PeopleLevel;
use bb_core::comment::{
    MENTION_RECIPIENTS_MAX, Mention, MentionTarget, handle_of, names_the_handle,
};
use bb_core::id::{CommentId, GroupId, RoleId, UserId};
use bb_core::issue::Issue;
use bb_core::people_kinds::PeopleKind;
use bb_core::readable_id::ReadableId;
use bb_markdown::{
    MAX_RESOLVED_ISSUE_IDS, MAX_RESOLVED_MENTIONS, Options, Token, TokenKind, render,
};
use bb_policy::{Evaluation, PeopleContext, row_reaches};

use crate::access::Actor;
use crate::auth::{AppError, AuthStore, Ctx, internal};

/// The most people a search for the person behind a handle reads: names that share the first word
/// beyond this many are not told apart.
pub(super) const PEOPLE_LOOK_AT: u64 = 100;

/// What a handle names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Named {
    /// A person.
    Person(UserId),
    /// A group.
    Group(GroupId),
    /// A role.
    Role(RoleId),
}

impl Named {
    const fn target(self) -> MentionTarget {
        match self {
            Self::Person(_) => MentionTarget::Person,
            Self::Group(_) => MentionTarget::Group,
            Self::Role(_) => MentionTarget::Role,
        }
    }
}

/// What the tokens of a text come to for one viewer.
#[derive(Debug, Default)]
pub(crate) struct Resolution {
    /// The places of the tokens that stay plain text.
    pub(crate) plain: Vec<usize>,
    /// The things the mentions name that could be told, each once.
    pub(crate) named: Vec<Named>,
}

impl<S: AuthStore> Ctx<'_, S> {
    /// What the handle names for a writer or reader who sees people of the kind *Mentions* at
    /// `level` in the project of `issue`: exactly one thing, or nothing. A person is named only if
    /// they may see the issue (`person_sees_issue`, the rule of every list of people on an issue),
    /// so a name never tells who belongs to a project whose members the writer may not see.
    async fn name_behind(
        &self,
        level: PeopleLevel,
        issue: &Issue,
        handle: &str,
    ) -> Result<Option<Named>, AppError> {
        let key = (issue.id, level, handle.to_owned());
        let known = self
            .known
            .handles
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&key)
            .copied();
        if let Some(known) = known {
            return Ok(known);
        }
        let named = self.look_up_handle(level, issue, handle).await?;
        self.known
            .handles
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key, named);
        Ok(named)
    }

    /// The handles of the groups and the roles, which no person can have too: a mention of such a
    /// handle names the group or the role.
    pub(crate) async fn reserved_handles(&self) -> Result<Vec<String>, AppError> {
        let groups = self.group_names().await?;
        let roles = self.role_names().await?;
        Ok(groups
            .iter()
            .map(|(_, name)| name.as_str())
            .chain(roles.iter().map(|(_, name)| name.as_str()))
            .filter_map(handle_of)
            .collect())
    }

    /// The names of the groups, read once for the request.
    async fn group_names(&self) -> Result<Arc<Vec<(GroupId, String)>>, AppError> {
        if let Some(known) = self
            .known
            .group_names
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
        {
            return Ok(known);
        }
        let names: Vec<(GroupId, String)> = self
            .store()
            .groups()
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|group| (group.id, group.name))
            .collect();
        let names = Arc::new(names);
        *self
            .known
            .group_names
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(Arc::clone(&names));
        Ok(names)
    }

    /// The names of the roles, read once for the request.
    async fn role_names(&self) -> Result<Arc<Vec<(RoleId, String)>>, AppError> {
        if let Some(known) = self
            .known
            .role_names
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
        {
            return Ok(known);
        }
        let names: Vec<(RoleId, String)> = self
            .store()
            .roles()
            .await
            .map_err(|error| internal::<S>(&error))?
            .into_iter()
            .map(|record| (record.role.id, record.role.name))
            .collect();
        let names = Arc::new(names);
        *self
            .known
            .role_names
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(Arc::clone(&names));
        Ok(names)
    }

    /// The one thing a handle names, asked of the store.
    async fn look_up_handle(
        &self,
        level: PeopleLevel,
        issue: &Issue,
        handle: &str,
    ) -> Result<Option<Named>, AppError> {
        let mut found: Vec<Named> = Vec::new();
        let failed = |error| internal::<S>(&error);
        if level == PeopleLevel::Names
            // The handle a person has now. One they gave up names nobody in a new text; an old
            // text stores the person it named, so it keeps naming them.
            && let Some(person) = self
                .store()
                .person_with_current_handle(&handle.to_lowercase())
                .await
                .map_err(failed)?
            && self.person_sees_issue(person, issue).await?
        {
            found.push(Named::Person(person));
        }
        if level != PeopleLevel::Hidden {
            for (id, name) in self.group_names().await?.iter() {
                if names_the_handle(handle, name) {
                    found.push(Named::Group(*id));
                }
            }
        }
        if level == PeopleLevel::RoleOnly {
            for (id, name) in self.role_names().await?.iter() {
                if names_the_handle(handle, name) {
                    found.push(Named::Role(*id));
                }
            }
        }
        Ok(match found.as_slice() {
            [only] => Some(*only),
            _ => None,
        })
    }

    /// Resolves the tokens of `text` for the actor, who has `held` in the project of `issue`'s
    /// place: which stay plain, and what the mentions name.
    pub(crate) async fn resolve_tokens(
        &self,
        actor: &Actor,
        held: &Evaluation,
        issue: &Issue,
        text: &str,
        tokens: &[Token],
    ) -> Result<Resolution, AppError> {
        let level = held.people_level(PeopleKind::Mentions, PeopleContext::Ordinary);
        let mut resolution = Resolution::default();
        let mut maker = None;
        let (mut mentions, mut issue_ids) = (0, 0);
        for (index, token) in tokens.iter().enumerate() {
            let written = &text[token.start..token.end];
            // Past the limits a token stays text, without a look-up: a hostile text cannot make a
            // page cost thousands of statements for everybody who reads it.
            let (count, most) = match token.kind {
                TokenKind::IssueId => (&mut issue_ids, MAX_RESOLVED_ISSUE_IDS),
                TokenKind::Mention => (&mut mentions, MAX_RESOLVED_MENTIONS),
            };
            *count += 1;
            if *count > most {
                resolution.plain.push(index);
                continue;
            }
            let named = match token.kind {
                TokenKind::IssueId => {
                    let known = self
                        .known
                        .issue_ids
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .get(&(actor.person.id, written.to_owned()))
                        .copied();
                    let seen = if let Some(known) = known {
                        known
                    } else {
                        let seen = match written.parse::<ReadableId>() {
                            Ok(_) => match self.find_issue(written).await? {
                                Some(found) => {
                                    if maker.is_none() {
                                        maker = Some(self.line_maker().await?);
                                    }
                                    match maker.as_mut() {
                                        Some(maker) => self
                                            .line_of(actor, maker, &found.issue)
                                            .await?
                                            .is_some(),
                                        None => false,
                                    }
                                }
                                None => false,
                            },
                            Err(_) => false,
                        };
                        self.known
                            .issue_ids
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .insert((actor.person.id, written.to_owned()), seen);
                        seen
                    };
                    if !seen {
                        resolution.plain.push(index);
                    }
                    continue;
                }
                TokenKind::Mention => {
                    let handle = written.trim_start_matches('@');
                    self.name_behind(level, issue, handle).await?
                }
            };
            match named {
                Some(named) => {
                    if !resolution.named.contains(&named) {
                        resolution.named.push(named);
                    }
                }
                None => resolution.plain.push(index),
            }
        }
        Ok(resolution)
    }

    /// The text as safe HTML for the actor: the tokens that resolve are chips, the rest is text.
    pub(crate) async fn html_of(
        &self,
        actor: &Actor,
        held: &Evaluation,
        issue: &Issue,
        text: &str,
    ) -> Result<String, AppError> {
        let image_hosts = self.image_hosts().await?;
        let first = render(
            text,
            &Options {
                image_hosts: image_hosts.clone(),
                ..Options::default()
            },
        );
        if first.tokens.is_empty() {
            return Ok(first.html);
        }
        let resolution = self
            .resolve_tokens(actor, held, issue, text, &first.tokens)
            .await?;
        if resolution.plain.is_empty() {
            return Ok(first.html);
        }
        Ok(render(
            text,
            &Options {
                image_hosts,
                plain_tokens: resolution.plain,
            },
        )
        .html)
    }

    /// The people a comment puts into *Mentions*: for each thing it names, the person, the members
    /// of the group, or the people who have the role where the issue is. Each person once, with
    /// the first thing that reached them; not the author; at most `MENTION_RECIPIENTS_MAX`.
    pub(crate) async fn mentions_of_text(
        &self,
        comment: CommentId,
        author: UserId,
        issue: &Issue,
        named: &[Named],
    ) -> Result<Vec<Mention>, AppError> {
        let failed = |error| internal::<S>(&error);
        // Everyone the comment reaches, each once with the first thing that reached them.
        let mut reached: Vec<(UserId, MentionTarget)> = Vec::new();
        let mut add = |person: UserId, via: MentionTarget| {
            if person != author && !reached.iter().any(|(there, _)| *there == person) {
                reached.push((person, via));
            }
        };
        let chain = self
            .store()
            .project_chain(issue.project)
            .await
            .map_err(failed)?
            .unwrap_or_default();
        for thing in named {
            let via = thing.target();
            match *thing {
                Named::Person(person) => add(person, via),
                Named::Group(group) => {
                    for member in self.store().group_members(group).await.map_err(failed)? {
                        add(member.person, via);
                    }
                }
                Named::Role(role) => {
                    for row in self
                        .store()
                        .assignments_of_role(role)
                        .await
                        .map_err(failed)?
                    {
                        if !row_reaches(row.project, row.applies_to, &chain) {
                            continue;
                        }
                        if let Some(person) = row.person {
                            add(person, via);
                        }
                        if let Some(group) = row.group {
                            for member in self.store().group_members(group).await.map_err(failed)? {
                                add(member.person, via);
                            }
                        }
                    }
                }
            }
        }
        // Only people who may see the issue get a mention: a name in a comment never puts the
        // comment, or the issue, into the *Mentions* of somebody who cannot see it. (Delivery by
        // e-mail in M4 checks again at the moment of sending, as access may change.)
        let mut mentions: Vec<Mention> = Vec::new();
        for (person, via) in reached {
            if mentions.len() >= MENTION_RECIPIENTS_MAX {
                break;
            }
            if self.person_sees_issue(person, issue).await? {
                mentions.push(Mention {
                    comment,
                    recipient: person,
                    via,
                    read_at: None,
                });
            }
        }
        Ok(mentions)
    }
}
