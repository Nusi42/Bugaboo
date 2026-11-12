//! What is special about a route at the edges the API description does not say: the quirks of the
//! rows of the table of examples.

use bb_app::access::Permission;

/// What a route does at the edges the API description does not say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::sign_in_tests::matrix) enum Quirk {
    /// The route answers a project that the actor cannot see with `resource.not-found` before it
    /// looks at the permission, also an actor who knows it exists (they hold *Decide who has which
    /// role* there): it is about what is inside the project.
    HidesUnseen,
    /// The route looks at the permission first, and then answers `resource.not-found` to somebody
    /// who holds it but does not see the project: changing what one cannot see is not possible.
    NeedsToSeeTheProject,
    /// The route lists what the actor may see: a person without the permission is not refused but
    /// gets what they see, which may be nothing (M2-F30). Only the routes named here may do that.
    FiltersWhatIsSeen,
    /// The route is about the target project through its query (`?project=`) and not through its
    /// address: it counts what the actor holds in that project, and a project the actor may not
    /// know about is `resource.not-found`, like one in the address.
    AtTheTargetProject,
    /// The call ends the session of whoever makes it, so they sign in again afterwards.
    EndsTheSession,
    /// The route names a thing that is not there (a passkey nobody added), so a person who is let
    /// in is told `resource.not-found`, which is not a refusal of the permission.
    NamesAMissingThing,
    /// The route finishes a sign-in that is half done (the second step): a session that is whole
    /// has nothing to finish, so it is told it is not signed in for this.
    FinishesAHalfDoneSignIn,
    /// The route may answer with this other refusal (a `401` or `403` with this code) to a person
    /// who is let in: the wrong password it asks for again, a code that is wrong. Any other
    /// refusal that the matrix does not expect fails, so a permitted actor cannot be refused for
    /// another reason without anybody noticing.
    AlsoAnswers(&'static str),
    /// The route names a record that belongs to the person who made it (an upload). The example
    /// names the actor's own where the actor may start one, so the route runs on a real record; for
    /// whoever may not, it names one that nobody made, and they are told `resource.not-found`, as
    /// for a record that is not there, whether the permission is theirs or not, because the record
    /// is looked up before the permission of the issue it belongs to (M4 WP-05).
    TheirOwn,
    /// The route is about an issue in the target project that somebody else reported (M3 WP-03): a
    /// person who does not see the project, or lacks *See issues* there, is told
    /// `resource.not-found`, as for an issue that is not there; a person who sees the issue and
    /// lacks the permission of the route is refused.
    AnIssueOfTheTarget,
    /// The route takes something out of the trash: a person who sees the issue and lacks the
    /// permission is told `resource.not-found`, as for something that is not in the trash, and
    /// not that they are refused (SEC-06).
    InTheTrash,
    /// The route lists the issues of the target project: anybody who sees the project gets a list
    /// (without *See issues*, of the issues they reported), and anybody else
    /// `resource.not-found`.
    ListsTheIssuesSeen,
    /// A route of *My settings*: the person's own choices, not their sign-in data, so an access
    /// key acts as the person: it reads them, and changes them only if it may change anything
    /// (WP-07, ADR-0025 decision 8).
    KeysActAsThePerson,
    /// The route lets in whoever is signed in and the use case judges the record it names; the
    /// example names the target project, so the person needs this permission there (and to see
    /// the project: one who does not is told it is not found, before the permission is looked at).
    OnTheTargetProject(Permission),
    /// The route lets in whoever is signed in and the use case judges the record it names, which
    /// needs this permission in the whole organization (a theme for everyone).
    AtTheOrganization(Permission),
    /// The route declares one permission and its use case also needs this one on the record it
    /// names (*Edit anyway* takes over editing a record only for a person who may change it).
    AlsoNeeds(Permission),
    /// The route lets in whoever holds this permission in at least one project, wherever that is
    /// (making an agent, M7-Q1); everybody else is refused.
    HeldInSomeProject(Permission),
    /// The route names a tag that the example puts on an issue of the target project: a person who
    /// holds the permission but does not read the issues there is told `resource.not-found`, as
    /// for a tag that is not there (SEC-06).
    NamesATagOfTheTarget,
    /// The route names an issue type or a group of labels that the example creates in the target
    /// project: a person who may not know that project is told `resource.not-found`, as for one
    /// that is not there (SEC-03, ADR-0027 amendment).
    NamesASubjectOfTheTarget,
    /// The route belongs to an open editor, a tab of a signed-in person: an access key is turned
    /// away from it, whatever the key may do (ADR-0019).
    SessionOnly,
    /// The route answers every signed-in actor and says in its answer what it did to each issue it
    /// was asked about (changing many issues, ISS-09): the issue of the example, in the target
    /// project, is left alone as *not found* when the actor does not see it, as *not allowed* when
    /// they see it and lack *Change issues*, and is changed otherwise. The matrix reads that from
    /// the answer, as a route that is let in says nothing else about the actor.
    SkipsWhatItMayNotChange,
    /// The route is about another person's working hours: to an actor who may neither see nor
    /// change them, the person answers like one that does not exist; an actor who may see them
    /// and lacks the permission of the route is refused (ADR-0035, decision 8).
    HidesThePerson,
}
