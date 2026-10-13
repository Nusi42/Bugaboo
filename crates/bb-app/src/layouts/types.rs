//! What the layout use cases take and give back: inputs that change something are named after what
//! they do; inputs that only read end in `Query` (ARCH-EVENTS). Everything given back is wrapped in
//! `Authorized<T>` by the use case.

use bb_core::fields::FieldKey;
use bb_core::id::{IssueTypeId, LayoutId, LockToken, RoleId};
use bb_core::layout::LayoutKind;
use bb_policy::Masked;
use serde::Serialize;

use crate::issues::IssueTypeRef;
use crate::projects::ProjectRef;

/// Ask which fields a form shows to the person who asks.
pub struct FormLayoutQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, by its ID or its short name.
    pub project: String,
    /// The issue type of the issue that is shown or made; none while no type is picked.
    pub issue_type: Option<IssueTypeId>,
    /// The issue page or the new-issue form.
    pub kind: LayoutKind,
}

/// Ask for a layout to edit.
pub struct LayoutEditorQuery {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, by its ID or its short name; none for the whole organization.
    pub project: Option<String>,
    /// The issue type; none for all types.
    pub issue_type: Option<IssueTypeId>,
    /// The issue page or the new-issue form.
    pub kind: LayoutKind,
}

/// Make the layout of a place, as a copy of the one that counts there now (*Customize this form*).
pub struct MakeLayout {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The project, by its ID or its short name; none for the whole organization.
    pub project: Option<String>,
    /// The issue type; none for all types.
    pub issue_type: Option<IssueTypeId>,
    /// The issue page or the new-issue form.
    pub kind: LayoutKind,
}

/// A field in a section as an editor sends it.
#[derive(Clone, Debug)]
pub struct FieldInput {
    /// The field, built-in or own.
    pub field: FieldKey,
    /// The roles it is shown to.
    pub roles: Vec<RoleId>,
}

/// A section as an editor sends it.
#[derive(Clone, Debug)]
pub struct SectionInput {
    /// The tab it is under; empty for none.
    pub tab: String,
    /// Its title, as typed.
    pub title: String,
    /// Its fields, in order.
    pub fields: Vec<FieldInput>,
}

/// Save the sections and fields of a layout, under the edit lock the person holds.
pub struct SaveLayout {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The layout.
    pub layout: LayoutId,
    /// The version the person looked at.
    pub version: i64,
    /// The token the edit lock was taken with.
    pub lock_token: LockToken,
    /// The sections, in order.
    pub sections: Vec<SectionInput>,
}

/// Take the layout of a place away: the place uses the layout it inherits again.
pub struct DeleteLayout {
    /// The secret of the session of the request.
    pub session_token: String,
    /// The layout.
    pub layout: LayoutId,
    /// The version the person looked at.
    pub version: i64,
}

/// A section of a form as one person sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FormSection {
    /// The tab it is under; empty for none.
    pub tab: String,
    /// Its title.
    pub title: String,
    /// The fields the person sees (or fills in) here, in order.
    pub fields: Vec<FieldKey>,
}

/// The layout of a form for one person: only what they may see, or fill in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FormLayout {
    /// The issue page or the new-issue form.
    pub kind: LayoutKind,
    /// The sections that have at least one field for the person.
    pub sections: Vec<FormSection>,
}

impl Masked for FormLayout {}

/// A field of a layout as the editor shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorField {
    /// The field, built-in or own.
    pub field: FieldKey,
    /// The name of an own field; a built-in field is named by its word.
    pub name: Option<String>,
    /// The roles the layout shows it to.
    pub roles: Vec<RoleId>,
    /// The roles for which field permissions win: they may not see the field (on the new-issue
    /// form: not change it), whatever the layout says.
    pub limited_for: Vec<RoleId>,
    /// An own field that must be filled in: it stays on the new-issue form for everybody who can
    /// fill it in.
    pub required: bool,
}

/// A section as the editor shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorSection {
    /// The tab it is under; empty for none.
    pub tab: String,
    /// Its title.
    pub title: String,
    /// Its fields, in order.
    pub fields: Vec<EditorField>,
}

/// A role the editor can tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorRole {
    /// The role.
    pub id: RoleId,
    /// Its name.
    pub name: String,
}

/// A layout row that exists at exactly the place that was asked about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayoutSlot {
    /// The layout, which is what the edit lock is about.
    pub id: LayoutId,
    /// Its version.
    pub version: i64,
}

/// Where the layout that counts at a place comes from, when it is not set there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InheritedFrom {
    /// It is set for the whole organization.
    pub whole_organization: bool,
    /// The project it is set for, when the person may know it; a project they cannot see is not
    /// named.
    pub project: Option<ProjectRef>,
    /// The issue type it is for; none for all types.
    pub issue_type: Option<IssueTypeRef>,
}

/// What an editor shows for a place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutEditor {
    /// The issue page or the new-issue form.
    pub kind: LayoutKind,
    /// The project of the place; none for the whole organization.
    pub project: Option<ProjectRef>,
    /// The issue type of the place; none for all types.
    pub issue_type: Option<IssueTypeRef>,
    /// The layout set at this place, if there is one. Without it the place uses another layout
    /// ([`Self::inherited_from`]) or the starting layout, and *Customize* makes one.
    pub layout: Option<LayoutSlot>,
    /// Where the layout that counts is set, when it is another place.
    pub inherited_from: Option<InheritedFrom>,
    /// The sections.
    pub sections: Vec<EditorSection>,
    /// The fields that can be added: placeable here and not in a section yet.
    pub available: Vec<EditorField>,
    /// The roles that can be ticked.
    pub roles: Vec<EditorRole>,
}

impl Masked for LayoutEditor {}
