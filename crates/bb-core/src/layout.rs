//! Layouts (CFG-01, ISS-02, ADR-0027): which fields the issue page and the new-issue form show,
//! in which sections and tabs, and for which roles.
//!
//! A layout is set for the organization or for a project, and optionally for one issue type. The
//! nearest place wins, and at one place the layout for the issue type wins over the layout for all
//! types ([`find`], which is `crate::lookup::nearest_row`). Until somebody customizes a form, no
//! row exists and the form is the starting layout ([`LayoutContent::starting`]), made from the one
//! definition below.
//!
//! A layout only says where a field shows and for which roles. It never widens what a role may do:
//! a field shows only when the layout lists it for one of the person's roles *and* the level of
//! the field lets them see it (to fill it in on the new-issue form: change it). That second half is
//! decided by the policy engine in `bb-app` (`layouts::effective`), not here.

use std::collections::BTreeSet;

use crate::fields::{Field, FieldKey};
use crate::id::{IssueTypeId, LayoutId, ProjectId, RoleId};
use crate::lookup::nearest_row;
use crate::message::{self, Message};
use crate::personal::Entity;

/// The most sections one layout has.
pub const SECTIONS_MAX: usize = 20;
/// The most fields one layout has, over all its sections: every own field there can be, and every
/// built-in field a form can place (the issue page has the most). Made from those two limits, so
/// the starting layout of an organization with the most own fields still fits.
pub const PLACED_MAX: usize = crate::own_fields::FIELDS_MAX + MOST_BUILT_IN;
/// The most built-in fields a form of any kind can place.
const MOST_BUILT_IN: usize = {
    let (page, new) = (
        built_in_fields(LayoutKind::IssuePage).len(),
        built_in_fields(LayoutKind::NewIssue).len(),
    );
    if page > new { page } else { new }
};
/// The most characters of the title of a section or of the name of a tab.
pub const TITLE_MAX: usize = 60;

words! {
    /// What a layout is for.
    pub enum LayoutKind {
        /// The new-issue form: which fields it asks for and from whom.
        NewIssue => "new-issue",
        /// The issue page: which fields it shows and for whom.
        IssuePage => "issue-page",
    }
}

words_in_api!(
    LayoutKind,
    "What a layout is for: the issue page or the new-issue form."
);

/// The fields of the catalogue that exist on an issue and can be placed on a form of this kind.
/// The title is always there and the type is always asked for, so neither is placed. A field of
/// the catalogue that issues do not have yet is not listed.
#[must_use]
pub const fn built_in_fields(kind: LayoutKind) -> &'static [Field] {
    match kind {
        LayoutKind::IssuePage => &[
            Field::Description,
            Field::Status,
            Field::Priority,
            Field::AssignedTo,
            Field::ReportedBy,
            Field::Tags,
            Field::Labels,
        ],
        LayoutKind::NewIssue => &[Field::Description, Field::Priority, Field::AssignedTo],
    }
}

/// One field in a section, and the roles it is shown to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedField {
    /// The field, built-in or own.
    pub field: FieldKey,
    /// The roles the layout shows it to.
    pub roles: Vec<RoleId>,
}

/// One section of a layout: a title, a tab it sits under, and its fields in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionContent {
    /// The tab the section is under; empty for a layout without tabs.
    pub tab: String,
    /// The title of the section; may be empty.
    pub title: String,
    /// The fields, in the order they are shown.
    pub fields: Vec<PlacedField>,
}

/// Why a layout cannot be stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// More sections or fields than a layout may have.
    TooBig,
    /// A title or a tab name is longer than [`TITLE_MAX`] characters.
    TitleTooLong,
    /// A field is placed twice.
    FieldTwice,
    /// A built-in field that this kind of layout cannot place.
    NotPlaceable,
}

/// The sections of a layout, from the first to the last.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LayoutContent {
    /// The sections, in order.
    pub sections: Vec<SectionContent>,
}

impl LayoutContent {
    /// The fields in all sections, in order.
    pub fn placed(&self) -> impl Iterator<Item = &PlacedField> {
        self.sections.iter().flat_map(|section| &section.fields)
    }

    /// Whether the layout can be stored for this kind. Own fields are judged by the caller, who
    /// knows which of them exist.
    ///
    /// # Errors
    /// Names the first rule that is broken.
    pub fn check(&self, kind: LayoutKind) -> Result<(), Problem> {
        if self.sections.len() > SECTIONS_MAX || self.placed().count() > PLACED_MAX {
            return Err(Problem::TooBig);
        }
        if self.sections.iter().any(|section| {
            section.title.chars().count() > TITLE_MAX || section.tab.chars().count() > TITLE_MAX
        }) {
            return Err(Problem::TitleTooLong);
        }
        let mut seen = BTreeSet::new();
        for placed in self.placed() {
            if !seen.insert(placed.field) {
                return Err(Problem::FieldTwice);
            }
            if let FieldKey::BuiltIn(field) = placed.field
                && !built_in_fields(kind).contains(&field)
            {
                return Err(Problem::NotPlaceable);
            }
        }
        Ok(())
    }

    /// The layout a form has until somebody customizes it: the fields the form has today, for every
    /// role. `own` are the own fields that reach the form, in the order they are offered in. The
    /// new-issue form starts short (the description and the priority); the issue page starts with
    /// everything.
    #[must_use]
    pub fn starting(kind: LayoutKind, own: &[FieldKey], roles: &[RoleId]) -> Self {
        let for_all = |field: FieldKey| PlacedField {
            field,
            roles: roles.to_vec(),
        };
        let section = |title: Message, fields: Vec<FieldKey>| SectionContent {
            tab: String::new(),
            title: title.render(),
            fields: fields.into_iter().map(for_all).collect(),
        };
        let built_in = |fields: &[Field]| -> Vec<FieldKey> {
            fields.iter().copied().map(FieldKey::BuiltIn).collect()
        };
        let sections = match kind {
            LayoutKind::IssuePage => vec![
                section(
                    message::LAYOUT_SECTION_MAIN.into(),
                    built_in(&[
                        Field::Description,
                        Field::Status,
                        Field::AssignedTo,
                        Field::Priority,
                    ]),
                ),
                section(
                    message::LAYOUT_SECTION_DETAILS.into(),
                    built_in(&[Field::ReportedBy, Field::Tags, Field::Labels])
                        .into_iter()
                        .chain(own.iter().copied())
                        .collect(),
                ),
            ],
            LayoutKind::NewIssue => vec![section(
                message::LAYOUT_SECTION_DETAILS.into(),
                built_in(&[Field::Description, Field::Priority]),
            )],
        };
        Self { sections }
    }

    /// The rows to store for this layout.
    #[must_use]
    pub fn rows(
        &self,
        layout: LayoutId,
    ) -> (Vec<LayoutSection>, Vec<LayoutField>, Vec<LayoutFieldRole>) {
        let (mut sections, mut fields, mut roles) = (Vec::new(), Vec::new(), Vec::new());
        for (section_at, section) in (0_i64..).zip(&self.sections) {
            sections.push(LayoutSection {
                layout,
                position: section_at,
                tab: section.tab.clone(),
                title: section.title.clone(),
            });
            for (field_at, placed) in (0_i64..).zip(&section.fields) {
                fields.push(LayoutField {
                    layout,
                    field: placed.field,
                    section: section_at,
                    position: field_at,
                });
                roles.extend(placed.roles.iter().map(|role| LayoutFieldRole {
                    layout,
                    field: placed.field,
                    role: *role,
                }));
            }
        }
        (sections, fields, roles)
    }

    /// The layout the stored rows make. Rows of one layout only; a field row whose section is gone
    /// is left out.
    #[must_use]
    pub fn from_rows(
        sections: &[LayoutSection],
        fields: &[LayoutField],
        roles: &[LayoutFieldRole],
    ) -> Self {
        let mut ordered: Vec<&LayoutSection> = sections.iter().collect();
        ordered.sort_by_key(|section| section.position);
        let sections = ordered
            .into_iter()
            .map(|section| {
                let mut inside: Vec<&LayoutField> = fields
                    .iter()
                    .filter(|field| field.section == section.position)
                    .collect();
                inside.sort_by_key(|field| field.position);
                SectionContent {
                    tab: section.tab.clone(),
                    title: section.title.clone(),
                    fields: inside
                        .into_iter()
                        .map(|field| PlacedField {
                            field: field.field,
                            roles: roles
                                .iter()
                                .filter(|row| row.field == field.field)
                                .map(|row| row.role)
                                .collect(),
                        })
                        .collect(),
                }
            })
            .collect();
        Self { sections }
    }
}

/// Which place a layout is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayoutPlace {
    /// The project, or the whole organization.
    pub project: Option<ProjectId>,
    /// The issue type, or all types.
    pub issue_type: Option<IssueTypeId>,
    /// The issue page or the new-issue form.
    pub kind: LayoutKind,
}

/// A layout that is stored: its header (where it is set) and what it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredLayout {
    /// The header.
    pub layout: Layout,
    /// The sections and fields.
    pub content: LayoutContent,
}

/// The layout that counts for a place whose projects from the top down are `chain` and for an
/// issue type, among the headers of the layouts of one kind (ADR-0027, decision 2): the same walk
/// as for field levels. `None` when no layout was made yet, and the starting layout counts.
#[must_use]
pub fn find<'a>(
    chain: &[ProjectId],
    issue_type: Option<IssueTypeId>,
    headers: &'a [Layout],
) -> Option<&'a Layout> {
    nearest_row(
        chain,
        issue_type,
        headers
            .iter()
            .map(|header| (header.project, header.issue_type, header)),
    )
    .map(|(_, _, header)| header)
}

/// The header of a layout: where it is set. Its sections and fields are rows of their own.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct Layout {
    /// The ID of the layout.
    #[not_personal]
    pub id: LayoutId,
    /// The project it is set for, with its sub-projects; none for the whole organization.
    #[not_personal]
    pub project: Option<ProjectId>,
    /// The issue type it is for; none for all types.
    #[not_personal]
    pub issue_type: Option<IssueTypeId>,
    /// The issue page or the new-issue form.
    #[not_personal]
    pub kind: LayoutKind,
    /// Counts up with every change, for the check that nobody changed it meanwhile (ADR-0019).
    #[not_personal]
    pub version: i64,
}

impl Layout {
    /// Where the layout is set.
    #[must_use]
    pub const fn place(&self) -> LayoutPlace {
        LayoutPlace {
            project: self.project,
            issue_type: self.issue_type,
            kind: self.kind,
        }
    }
}

/// A section of a layout.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct LayoutSection {
    /// The layout.
    #[not_personal]
    pub layout: LayoutId,
    /// The place of the section among the others.
    #[not_personal]
    pub position: i64,
    /// The tab it sits under; free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub tab: String,
    /// Its title; free text that could name a person.
    #[personal(category = "content", basis = "legitimate_interests")]
    pub title: String,
}

/// A field in a section of a layout.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct LayoutField {
    /// The layout.
    #[not_personal]
    pub layout: LayoutId,
    /// The field, built-in or own.
    #[not_personal]
    pub field: FieldKey,
    /// The section it is in (its position).
    #[not_personal]
    pub section: i64,
    /// Its place in the section.
    #[not_personal]
    pub position: i64,
}

/// A role a field of a layout is shown to.
#[derive(Clone, Debug, PartialEq, Eq, Entity)]
pub struct LayoutFieldRole {
    /// The layout.
    #[not_personal]
    pub layout: LayoutId,
    /// The field.
    #[not_personal]
    pub field: FieldKey,
    /// The role.
    #[not_personal]
    pub role: RoleId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::SequentialIds;

    fn ids() -> (LayoutId, RoleId, RoleId) {
        let mut source = SequentialIds::default();
        (
            LayoutId::generate(&mut source).unwrap(),
            RoleId::generate(&mut source).unwrap(),
            RoleId::generate(&mut source).unwrap(),
        )
    }

    #[test]
    fn the_starting_layout_of_the_most_own_fields_is_one_a_save_accepts() {
        let mut source = SequentialIds::default();
        let own: Vec<FieldKey> = (0..crate::own_fields::FIELDS_MAX)
            .map(|_| FieldKey::Own(crate::id::FieldId::generate(&mut source).unwrap()))
            .collect();
        let role = RoleId::generate(&mut source).unwrap();
        for kind in [LayoutKind::IssuePage, LayoutKind::NewIssue] {
            let starting = LayoutContent::starting(kind, &own, &[role]);
            assert_eq!(starting.check(kind), Ok(()), "{kind:?}");
        }
        // One field more than there can be is too many.
        let mut more = LayoutContent::starting(LayoutKind::IssuePage, &own, &[role]);
        more.sections[0].fields.push(PlacedField {
            field: FieldKey::Own(crate::id::FieldId::generate(&mut source).unwrap()),
            roles: vec![],
        });
        assert_eq!(more.check(LayoutKind::IssuePage), Err(Problem::TooBig));
    }

    #[test]
    fn rows_and_content_make_each_other() {
        let (layout, team, guest) = ids();
        let content = LayoutContent {
            sections: vec![
                SectionContent {
                    tab: "Basics".into(),
                    title: "Main".into(),
                    fields: vec![
                        PlacedField {
                            field: Field::Priority.into(),
                            roles: vec![team, guest],
                        },
                        PlacedField {
                            field: Field::Description.into(),
                            roles: vec![team],
                        },
                    ],
                },
                SectionContent {
                    tab: String::new(),
                    title: String::new(),
                    fields: vec![],
                },
            ],
        };
        let (sections, fields, roles) = content.rows(layout);
        assert_eq!(
            LayoutContent::from_rows(&sections, &fields, &roles),
            content
        );
    }

    #[test]
    fn a_field_is_placed_once_and_only_where_the_form_can_take_it() {
        let (_, team, _) = ids();
        let placed = |field: Field| PlacedField {
            field: field.into(),
            roles: vec![team],
        };
        let one = |fields: Vec<PlacedField>| LayoutContent {
            sections: vec![SectionContent {
                tab: String::new(),
                title: String::new(),
                fields,
            }],
        };
        assert_eq!(
            one(vec![placed(Field::Priority), placed(Field::Priority)]).check(LayoutKind::NewIssue),
            Err(Problem::FieldTwice)
        );
        // The issue page shows *Reported by*; nobody fills it in on the new-issue form.
        assert_eq!(
            one(vec![placed(Field::ReportedBy)]).check(LayoutKind::NewIssue),
            Err(Problem::NotPlaceable)
        );
        assert_eq!(
            one(vec![placed(Field::ReportedBy)]).check(LayoutKind::IssuePage),
            Ok(())
        );
        let long = "x".repeat(TITLE_MAX + 1);
        let mut titled = one(vec![]);
        titled.sections[0].title = long;
        assert_eq!(
            titled.check(LayoutKind::IssuePage),
            Err(Problem::TitleTooLong)
        );
    }

    #[test]
    fn the_starting_layouts_are_for_every_role_and_the_form_starts_short() {
        let (_, team, guest) = ids();
        let form = LayoutContent::starting(LayoutKind::NewIssue, &[], &[team, guest]);
        assert_eq!(form.check(LayoutKind::NewIssue), Ok(()));
        let fields: Vec<_> = form.placed().map(|placed| placed.field).collect();
        assert_eq!(
            fields,
            vec![
                FieldKey::from(Field::Description),
                FieldKey::from(Field::Priority)
            ]
        );
        assert!(form.placed().all(|placed| placed.roles == [team, guest]));
        let page = LayoutContent::starting(LayoutKind::IssuePage, &[], &[team]);
        assert_eq!(page.check(LayoutKind::IssuePage), Ok(()));
        assert!(page.placed().count() > form.placed().count());
    }

    #[test]
    fn adr_0027_the_layout_of_the_nearest_place_wins() {
        let mut source = SequentialIds::default();
        let parent = ProjectId::generate(&mut source).unwrap();
        let child = ProjectId::generate(&mut source).unwrap();
        let bug = IssueTypeId::generate(&mut source).unwrap();
        let mut header = |project, issue_type| Layout {
            id: LayoutId::generate(&mut source).unwrap(),
            project,
            issue_type,
            kind: LayoutKind::NewIssue,
            version: 1,
        };
        let headers = vec![
            header(None, None),
            header(Some(parent), None),
            header(Some(parent), Some(bug)),
        ];
        let chain = [parent, child];
        // The child has none of its own: the parent's layout for bugs counts for bugs, the
        // parent's layout for all types for the rest.
        assert_eq!(find(&chain, Some(bug), &headers), Some(&headers[2]));
        assert_eq!(find(&chain, None, &headers), Some(&headers[1]));
        assert_eq!(find(&[], None, &headers), Some(&headers[0]));
        assert_eq!(find(&chain, None, &[]), None);
    }
}
