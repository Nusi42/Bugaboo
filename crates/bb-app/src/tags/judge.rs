//! The rules for putting tags and labels on an issue, as plain functions of what is stored: the
//! same function judges a request before it is written and again inside the change, after the lock,
//! so the answer of the preview and the result of the change cannot differ (ISS-09, TAG-01,
//! TAG-02).

use super::refs::NameRef;
use crate::offers::GroupOffers;
use bb_core::ErrorCode;
use bb_core::id::{LabelId, ProjectId, TagId};
use bb_core::names::same_name;
use bb_core::tags::{
    IssueLabel, IssueTag, LABELS_PER_ISSUE_MAX, Label, LabelGroup, TAGS_PER_ISSUE_MAX, Tag,
    TagProject, tag_allowed_in,
};

/// The names of the tags an issue has, in the order it got them.
pub(super) fn tag_names(tags: &[Tag], links: &[IssueTag]) -> Vec<String> {
    links
        .iter()
        .filter_map(|link| tags.iter().find(|tag| tag.id == link.tag))
        .map(|tag| tag.name.clone())
        .collect()
}

/// The labels an issue has, in the order it got them, with their names and groups.
pub(super) fn label_refs(labels: &[Label], links: &[IssueLabel]) -> Vec<NameRef> {
    links
        .iter()
        .filter_map(|link| labels.iter().find(|label| label.id == link.label))
        .map(NameRef::from)
        .collect()
}

/// What putting a tag on an issue does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TagAddition {
    /// The tag, when there is one with the name already; the tag is made when there is none.
    pub(super) tag: Option<TagId>,
    /// The issue has the tag already.
    pub(super) already: bool,
}

/// Whether a tag may be put on an issue in a project with this chain of parents, and what it does.
///
/// # Errors
/// `tag.not-here` for a tag limited to other projects, `limit.reached` for an issue that has as
/// many tags as it may.
pub(super) fn judge_tag(
    name: &str,
    tags: &[Tag],
    limits: &[TagProject],
    held: &[IssueTag],
    chain: &[ProjectId],
) -> Result<TagAddition, ErrorCode> {
    let found = tags.iter().find(|tag| same_name(&tag.name, name));
    let Some(tag) = found else {
        return if held.len() >= TAGS_PER_ISSUE_MAX {
            Err(ErrorCode::LimitReached)
        } else {
            Ok(TagAddition {
                tag: None,
                already: false,
            })
        };
    };
    if !tag_allowed_in(tag.id, limits, chain) {
        return Err(ErrorCode::TagNotHere);
    }
    let already = held.iter().any(|link| link.tag == tag.id);
    if !already && held.len() >= TAGS_PER_ISSUE_MAX {
        return Err(ErrorCode::LimitReached);
    }
    Ok(TagAddition {
        tag: Some(tag.id),
        already,
    })
}

/// What putting a label on an issue does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LabelAddition {
    /// The labels of a *Pick one* group that the new label takes the place of.
    pub(super) replaces: Vec<LabelId>,
    /// The issue has the label already.
    pub(super) already: bool,
}

/// Whether a label may be put on an issue that has these labels, and what it does.
///
/// # Errors
/// `resource.not-found` for a label that is not there, `request.invalid` for a retired label or a
/// label of a retired group, `label.not-offered` for a label of a group that the project of the
/// issue does not offer, `limit.reached` for an issue that has as many labels as it may.
pub(super) fn judge_label(
    wanted: LabelId,
    labels: &[Label],
    groups: &[LabelGroup],
    held: &[IssueLabel],
    offers: &GroupOffers,
) -> Result<LabelAddition, ErrorCode> {
    let label = labels
        .iter()
        .find(|label| label.id == wanted)
        .ok_or(ErrorCode::NotFound)?;
    let group = label
        .group
        .and_then(|id| groups.iter().find(|group| group.id == id));
    let already = held.iter().any(|link| link.label == wanted);
    if already {
        return Ok(LabelAddition {
            replaces: Vec::new(),
            already: true,
        });
    }
    if label.retired_at.is_some() || group.is_some_and(|group| group.retired_at.is_some()) {
        return Err(ErrorCode::InvalidRequest);
    }
    if group.is_some_and(|group| !offers.offers(group)) {
        return Err(ErrorCode::LabelNotOffered);
    }
    let replaces: Vec<LabelId> = match group.filter(|group| group.pick_one) {
        Some(group) => held
            .iter()
            .filter(|link| {
                labels
                    .iter()
                    .any(|other| other.id == link.label && other.group == Some(group.id))
            })
            .map(|link| link.label)
            .collect(),
        None => Vec::new(),
    };
    if held.len() - replaces.len() >= LABELS_PER_ISSUE_MAX {
        return Err(ErrorCode::LimitReached);
    }
    Ok(LabelAddition {
        replaces,
        already: false,
    })
}

#[cfg(test)]
mod tests {
    use bb_core::id::{IdSource, IssueId, LabelGroupId, SequentialIds, UserId};
    use bb_core::own_fields::ChoiceColor;
    use bb_core::version::FIRST_VERSION;
    use jiff::Timestamp;

    use super::*;

    fn at() -> Timestamp {
        Timestamp::from_second(1_767_225_600).unwrap()
    }

    fn label(ids: &mut impl IdSource, group: Option<LabelGroupId>, name: &str) -> Label {
        Label {
            id: LabelId::generate(ids).unwrap(),
            group,
            name: name.to_owned(),
            color: ChoiceColor::Cat1,
            position: 0,
            retired_at: None,
            version: FIRST_VERSION,
        }
    }

    fn group(ids: &mut impl IdSource, pick_one: bool) -> LabelGroup {
        LabelGroup {
            project: None,
            id: LabelGroupId::generate(ids).unwrap(),
            name: "Area".to_owned(),
            pick_one,
            position: 0,
            retired_at: None,
            version: FIRST_VERSION,
        }
    }

    fn on(issue: IssueId, label: &Label) -> IssueLabel {
        IssueLabel {
            issue,
            label: label.id,
            added_by: UserId::generate(&mut SequentialIds::default()),
            added_at: at(),
        }
    }

    #[test]
    fn tag_02_a_label_of_a_pick_one_group_takes_the_place_of_the_one_the_issue_has() {
        let mut ids = SequentialIds::default();
        let issue = IssueId::generate(&mut ids).unwrap();
        let area = group(&mut ids, true);
        let (login, billing, loose) = (
            label(&mut ids, Some(area.id), "login"),
            label(&mut ids, Some(area.id), "billing"),
            label(&mut ids, None, "urgent"),
        );
        let labels = [login.clone(), billing.clone(), loose.clone()];
        let held = [on(issue, &login), on(issue, &loose)];
        let judged = judge_label(
            billing.id,
            &labels,
            std::slice::from_ref(&area),
            &held,
            &everywhere(),
        )
        .unwrap();
        assert_eq!(judged.replaces, vec![login.id]);
        assert!(!judged.already);
        // A label that sits in no group, and a group that is not *Pick one*, replace nothing.
        let more = label(&mut ids, None, "more");
        let mut labels = labels.to_vec();
        labels.push(more.clone());
        assert!(
            judge_label(
                more.id,
                &labels,
                std::slice::from_ref(&area),
                &held,
                &everywhere(),
            )
            .unwrap()
            .replaces
            .is_empty()
        );
        let several = LabelGroup {
            project: None,
            pick_one: false,
            ..area.clone()
        };
        assert!(
            judge_label(billing.id, &labels, &[several], &held, &everywhere())
                .unwrap()
                .replaces
                .is_empty()
        );
    }

    #[test]
    fn tag_02_a_label_of_a_group_the_project_does_not_offer_is_not_put_on_an_issue() {
        let mut ids = SequentialIds::default();
        let issue = IssueId::generate(&mut ids).unwrap();
        let (here, elsewhere) = (
            ProjectId::generate(&mut ids).unwrap(),
            ProjectId::generate(&mut ids).unwrap(),
        );
        let mut area = group(&mut ids, false);
        area.project = Some(elsewhere);
        let kept = label(&mut ids, Some(area.id), "login");
        let labels = [kept.clone()];
        let offers = GroupOffers::new(Vec::new(), vec![here]);
        assert_eq!(
            judge_label(kept.id, &labels, std::slice::from_ref(&area), &[], &offers),
            Err(ErrorCode::LabelNotOffered)
        );
        // A label the issue has stays on it, shown like a retired one.
        let held = [on(issue, &kept)];
        assert!(
            judge_label(
                kept.id,
                &labels,
                std::slice::from_ref(&area),
                &held,
                &offers
            )
            .unwrap()
            .already
        );
        // Made in the project itself, the group is offered there.
        area.project = Some(here);
        assert!(judge_label(kept.id, &labels, &[area], &[], &offers).is_ok());
    }

    fn everywhere() -> GroupOffers {
        GroupOffers::new(Vec::new(), Vec::new())
    }

    #[test]
    fn tag_02_a_retired_label_is_not_put_on_an_issue_but_stays_where_it_is() {
        let mut ids = SequentialIds::default();
        let issue = IssueId::generate(&mut ids).unwrap();
        let mut old = label(&mut ids, None, "old");
        old.retired_at = Some(at());
        let labels = [old.clone()];
        assert_eq!(
            judge_label(old.id, &labels, &[], &[], &everywhere()),
            Err(ErrorCode::InvalidRequest)
        );
        let held = [on(issue, &old)];
        assert!(
            judge_label(old.id, &labels, &[], &held, &everywhere())
                .unwrap()
                .already
        );
    }

    #[test]
    fn tag_01_a_tag_for_other_projects_is_not_put_on_an_issue_here() {
        let mut ids = SequentialIds::default();
        let (here, elsewhere) = (
            ProjectId::generate(&mut ids).unwrap(),
            ProjectId::generate(&mut ids).unwrap(),
        );
        let tag = Tag {
            id: TagId::generate(&mut ids).unwrap(),
            name: "Billing".to_owned(),
            color: ChoiceColor::Cat1,
            created_at: at(),
            version: FIRST_VERSION,
        };
        let limits = [TagProject {
            tag: tag.id,
            project: elsewhere,
        }];
        let tags = [tag.clone()];
        assert_eq!(
            judge_tag("billing", &tags, &limits, &[], &[here]),
            Err(ErrorCode::TagNotHere)
        );
        let judged = judge_tag("BILLING", &tags, &limits, &[], &[elsewhere, here]).unwrap();
        assert_eq!(judged.tag, Some(tag.id));
        // A name nobody used makes a new tag.
        assert_eq!(
            judge_tag("fresh", &tags, &limits, &[], &[here])
                .unwrap()
                .tag,
            None
        );
    }
}
