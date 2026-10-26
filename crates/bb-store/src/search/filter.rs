//! The tree of a search as a condition: `and`, `or`, `not`, the tests of a field and the helpers
//! that follow sub-issues and relations (QRY-04).

use bb_core::names::name_key;
use bb_searchlang::{Expr as Node, MAX_STEPS, Test};
use sea_query::{
    Alias, CommonTableExpression, Cond, Expr, ExprTrait as _, IntoIden as _, Query,
    SelectStatement, UnionType,
};

use super::source::{Domain, Subject};
use super::{CompileError, Compiler, nothing};
use crate::db::StoredId as _;
use crate::schema::{GroupMemberRow, IssueParentRow, IssueRow, RelationRow};

impl Compiler<'_> {
    /// The condition a node of the tree stands for.
    pub(super) fn filter(&self, node: &Node) -> Result<Cond, CompileError> {
        match node {
            Node::Or { items, .. } => items
                .iter()
                .try_fold(Cond::any(), |cond, item| Ok(cond.add(self.filter(item)?))),
            Node::And { items, .. } => items
                .iter()
                .try_fold(Cond::all(), |cond, item| Ok(cond.add(self.filter(item)?))),
            Node::Not { item, .. } => Ok(self.filter(item)?.not()),
            Node::Field { field, test, .. } => self.field_test(&field.word, test),
            Node::Under { id, .. } => Ok(self.under(&id.text)),
            Node::Blocks { id, .. } => Ok(self.blocking(&id.text, Way::Blocks)),
            Node::BlockedBy { id, .. } => Ok(self.blocking(&id.text, Way::BlockedBy)),
            Node::ConnectedTo { id, steps, .. } => {
                let limit = steps.as_ref().map_or(MAX_STEPS, |steps| steps.count);
                Ok(self.connected(&id.text, limit))
            }
        }
    }

    fn field_test(&self, word: &str, test: &Test) -> Result<Cond, CompileError> {
        let target = self.target(word)?;
        if let Test::Contains { text, .. } = test {
            return self.contains(target, text);
        }
        let (subject, domain) = self.subject_of(target);
        match test {
            Test::Is { value, .. } => self.equals_in(&subject, domain, value),
            Test::IsNot { value, .. } => self.is_not(&subject, domain, std::slice::from_ref(value)),
            Test::In { values, .. } => {
                let mut any = Cond::any();
                for value in values {
                    any = any.add(self.equals_in(&subject, domain, value)?);
                }
                Ok(any)
            }
            Test::Compare {
                operator, value, ..
            } => {
                let cond = self.compares_in(&subject, domain, *operator, value)?;
                Ok(cond)
            }
            Test::IsEmpty { .. } => Ok(Self::empty(&subject, domain)),
            Test::IsNotEmpty { .. } => Ok(Self::empty(&subject, domain).not()),
            Test::InLast { count, unit, .. } => {
                let Domain::Time(clock) = domain else {
                    return Err(CompileError::Unsupported(
                        "in last on a field that is no time",
                    ));
                };
                let range = self.last(*count, *unit, clock)?;
                Ok(subject.holds(|each| {
                    Cond::all()
                        .add(each.clone().gte(range.from))
                        .add(each.lt(range.to))
                }))
            }
            Test::Contains { .. } => Err(CompileError::Unsupported("contains is made above")),
            Test::MemberOf { group, .. } => {
                let key = name_key(&group.text);
                let groups: Vec<_> = self
                    .request
                    .names
                    .member_groups
                    .iter()
                    .filter(|(name, _)| *name == key)
                    .flat_map(|(_, ids)| ids.iter().map(|id| id.stored()))
                    .collect();
                Ok(subject.holds(|each| {
                    if groups.is_empty() {
                        return nothing();
                    }
                    Cond::all().add(
                        each.in_subquery(
                            Query::select()
                                .column(GroupMemberRow::Person)
                                .from(GroupMemberRow::Table)
                                .and_where(Expr::col(GroupMemberRow::Group).is_in(groups.clone()))
                                .to_owned(),
                        ),
                    )
                }))
            }
            Test::Within { project, .. } => Ok(self.within(&subject, &project.text)),
        }
    }

    fn equals_in(
        &self,
        subject: &Subject,
        domain: Domain,
        value: &bb_searchlang::Operand,
    ) -> Result<Cond, CompileError> {
        self.check(domain, value)?;
        Ok(subject.holds(|each| {
            self.equals(domain, each, value)
                .unwrap_or_else(|_| nothing())
        }))
    }

    fn compares_in(
        &self,
        subject: &Subject,
        domain: Domain,
        operator: bb_searchlang::Comparison,
        value: &bb_searchlang::Operand,
    ) -> Result<Cond, CompileError> {
        // A comparison that cannot be made is reported once, before any condition is built.
        self.compares(domain, Expr::val(0), operator, value)?;
        Ok(subject.holds(|each| {
            self.compares(domain, each, operator, value)
                .unwrap_or_else(|_| nothing())
        }))
    }

    fn is_not(
        &self,
        subject: &Subject,
        domain: Domain,
        values: &[bb_searchlang::Operand],
    ) -> Result<Cond, CompileError> {
        let mut any = Cond::any();
        for value in values {
            self.check(domain, value)?;
            any = any.add(subject.holds(|each| {
                self.equals(domain, each, value)
                    .unwrap_or_else(|_| nothing())
            }));
        }
        Ok(match subject {
            Subject::Column(value) => Cond::any().add(value.clone().is_null()).add(any.not()),
            Subject::Linked(..) => any.not(),
        })
    }

    /// Whether the value can be compared at all: a failure that would otherwise hide in a closure.
    fn check(&self, domain: Domain, value: &bb_searchlang::Operand) -> Result<(), CompileError> {
        self.equals(domain, Expr::val(0), value).map(|_| ())
    }

    fn empty(subject: &Subject, domain: Domain) -> Cond {
        match (subject, domain) {
            // A text may be written as nothing, which is as empty as a missing one.
            (Subject::Column(value), Domain::Text) => Cond::any()
                .add(value.clone().is_null())
                .add(value.clone().eq("")),
            _ => subject.empty(),
        }
    }

    /// The project is the named one or inside it.
    fn within(&self, subject: &Subject, short_name: &str) -> Cond {
        let inside = self.subtree(short_name);
        subject.holds(|each| {
            if inside.is_empty() {
                nothing()
            } else {
                Cond::all().add(each.is_in(inside.iter().map(|project| project.stored())))
            }
        })
    }

    /// The project with this short name and the projects below it that the person sees.
    fn subtree(&self, short_name: &str) -> Vec<bb_core::id::ProjectId> {
        let projects = &self.request.projects;
        let mut found: Vec<_> = self
            .project_named(short_name)
            .map(|root| root.id)
            .into_iter()
            .collect();
        let mut at = 0;
        while at < found.len() {
            let parent = found[at];
            let below: Vec<_> = projects
                .iter()
                .filter(|project| project.parent == Some(parent) && !found.contains(&project.id))
                .map(|project| project.id)
                .collect();
            found.extend(below);
            at += 1;
        }
        found
    }

    /// Issues below the named one, at any depth, through issues the person reads.
    fn under(&self, reference: &str) -> Cond {
        let read = crate::sight::sight_condition(&self.request.sight);
        let mut first = Query::select();
        first
            .column((IssueParentRow::Table, IssueParentRow::Child))
            .from(IssueParentRow::Table)
            .inner_join(
                IssueRow::Table,
                Expr::col((IssueRow::Table, IssueRow::Id))
                    .equals((IssueParentRow::Table, IssueParentRow::Child)),
            )
            .and_where(
                Expr::col((IssueParentRow::Table, IssueParentRow::Parent))
                    .in_subquery(self.issues_of(reference)),
            )
            .cond_where(read.clone());
        let name = Alias::new(format!("walk{}", self.next_table()));
        let mut next = Query::select();
        next.column((IssueParentRow::Table, IssueParentRow::Child))
            .from(IssueParentRow::Table)
            .inner_join(
                name.clone(),
                Expr::col((name.clone(), Alias::new("id")))
                    .equals((IssueParentRow::Table, IssueParentRow::Parent)),
            )
            .inner_join(
                IssueRow::Table,
                Expr::col((IssueRow::Table, IssueRow::Id))
                    .equals((IssueParentRow::Table, IssueParentRow::Child)),
            )
            .cond_where(read);
        first.union(UnionType::Distinct, next);
        self.keep_walk(&name, first, &["id"]);
        Cond::all().add(Expr::col((IssueRow::Table, IssueRow::Id)).in_subquery(walk_ids(&name)))
    }

    /// Issues that end a relation of a blocking kind that starts or ends at the named one.
    fn blocking(&self, reference: &str, way: Way) -> Cond {
        if self.request.blocking_kinds.is_empty() {
            return nothing();
        }
        let (me, other) = match way {
            Way::Blocks => (RelationRow::FromIssue, RelationRow::ToIssue),
            Way::BlockedBy => (RelationRow::ToIssue, RelationRow::FromIssue),
        };
        Cond::all().add(
            Expr::col((IssueRow::Table, IssueRow::Id)).in_subquery(
                Query::select()
                    .column(me)
                    .from(RelationRow::Table)
                    .and_where(
                        Expr::col(RelationRow::Kind)
                            .is_in(self.request.blocking_kinds.iter().map(|kind| kind.stored())),
                    )
                    .and_where(Expr::col(other).in_subquery(self.issues_of(reference)))
                    .to_owned(),
            ),
        )
    }

    /// Issues that a chain of relations of at most `limit` steps joins to the named one, through
    /// issues the person reads.
    fn connected(&self, reference: &str, limit: u32) -> Cond {
        let read = crate::sight::sight_condition(&self.request.sight);
        let name = Alias::new(format!("walk{}", self.next_table()));
        let at = |column: &str| Expr::col((name.clone(), Alias::new(column.to_owned())));
        let mut first = Query::select();
        first
            .column((IssueRow::Table, IssueRow::Id))
            .expr(Expr::val(0))
            .from(IssueRow::Table)
            .and_where(
                Expr::col((IssueRow::Table, IssueRow::Id)).in_subquery(self.issues_of(reference)),
            );
        let far_end = Expr::case(
            Expr::col((RelationRow::Table, RelationRow::FromIssue))
                .equals((name.clone(), Alias::new("id"))),
            Expr::col((RelationRow::Table, RelationRow::ToIssue)),
        )
        .finally(Expr::col((RelationRow::Table, RelationRow::FromIssue)));
        let mut next = Query::select();
        next.expr(far_end.clone())
            .expr(at("depth").add(1))
            .from(name.clone())
            .inner_join(
                RelationRow::Table,
                Expr::col((RelationRow::Table, RelationRow::FromIssue))
                    .equals((name.clone(), Alias::new("id")))
                    .or(Expr::col((RelationRow::Table, RelationRow::ToIssue))
                        .equals((name.clone(), Alias::new("id")))),
            )
            .and_where(at("depth").lt(i64::from(limit)))
            .and_where(
                Expr::expr(far_end).in_subquery(
                    Query::select()
                        .column((IssueRow::Table, IssueRow::Id))
                        .from(IssueRow::Table)
                        .cond_where(read)
                        .to_owned(),
                ),
            );
        first.union(UnionType::Distinct, next);
        self.keep_walk(&name, first, &["id", "depth"]);
        Cond::all()
            .add(
                Expr::col((IssueRow::Table, IssueRow::Id)).in_subquery(
                    Query::select()
                        .column(Alias::new("id"))
                        .from(name)
                        .and_where(Expr::col(Alias::new("depth")).gt(0))
                        .to_owned(),
                ),
            )
            .add(
                Expr::col((IssueRow::Table, IssueRow::Id))
                    .in_subquery(self.issues_of(reference))
                    .not(),
            )
    }

    /// Keeps the table of a walk for the head of the statement.
    fn keep_walk(&self, name: &Alias, query: SelectStatement, columns: &[&str]) {
        let mut table = CommonTableExpression::new();
        table
            .table_name(name.clone())
            .columns(
                columns
                    .iter()
                    .map(|column| Alias::new((*column).to_owned())),
            )
            .query(query);
        self.keep(table);
    }
}

fn walk_ids(name: &Alias) -> SelectStatement {
    Query::select()
        .column(Alias::new("id"))
        .from(name.clone().into_iden())
        .to_owned()
}

/// Which end of a relation the named issue is.
#[derive(Clone, Copy)]
enum Way {
    /// The issues that block it.
    Blocks,
    /// The issues it blocks.
    BlockedBy,
}
