use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::fields::Field;
use crate::id::{FieldId, GroupId, RoleId, SequentialIds};
use crate::workflow::starter::{DONE, IN_PROGRESS, OPEN, SIMPLE};
use crate::workflow::{Condition, WhoCan};

/// What the judge is told about the fields, written out.
#[derive(Default)]
struct Facts {
    levels: BTreeMap<FieldKey, FieldLevel>,
    filled: BTreeSet<FieldKey>,
    given: BTreeSet<FieldKey>,
}

impl Facts {
    fn can(mut self, field: FieldKey, level: FieldLevel) -> Self {
        self.levels.insert(field, level);
        self
    }

    fn with_value(mut self, field: FieldKey) -> Self {
        self.filled.insert(field);
        self
    }

    fn carrying(mut self, field: FieldKey) -> Self {
        self.given.insert(field);
        self
    }
}

impl FieldFacts for Facts {
    fn level(&self, field: FieldKey) -> FieldLevel {
        self.levels
            .get(&field)
            .copied()
            .unwrap_or(FieldLevel::Hidden)
    }

    fn filled(&self, field: FieldKey) -> bool {
        self.filled.contains(&field)
    }

    fn given(&self, field: FieldKey) -> bool {
        self.given.contains(&field)
    }
}

fn key(text: &str) -> StatusKey {
    StatusKey::new(text).unwrap()
}

fn customer() -> FieldKey {
    let mut source = SequentialIds::default();
    FieldKey::Own(FieldId::generate(&mut source).unwrap())
}

fn description() -> FieldKey {
    FieldKey::BuiltIn(Field::Description)
}

fn anyone() -> Changer<'static> {
    Changer {
        may_change_status: true,
        roles: &[],
        groups: &[],
        assigned: false,
        reporter: false,
    }
}

/// *Simple* with the rules on its change from *In progress* to *Done*.
fn simple_with(rules: StatusRules) -> WorkflowContent {
    let mut workflow = SIMPLE.content();
    let change = workflow
        .changes
        .iter_mut()
        .find(|each| each.from == key(IN_PROGRESS) && each.to == key(DONE))
        .unwrap();
    change.rules = rules;
    workflow
}

fn judged(workflow: &WorkflowContent, changer: &Changer<'_>, facts: &Facts) -> Verdict {
    judge(workflow, &key(IN_PROGRESS), &key(DONE), changer, facts)
}

fn refused(why: Why, field: Option<FieldKey>) -> Verdict {
    Verdict::NotAllowed(Because { why, field })
}

#[test]
fn wf_03_without_rules_every_status_change_of_the_workflow_is_allowed() {
    let workflow = SIMPLE.content();
    assert_eq!(
        judged(&workflow, &anyone(), &Facts::default()),
        Verdict::Allowed
    );
}

#[test]
fn wf_01_a_status_change_the_workflow_does_not_have_is_not_allowed() {
    let mut workflow = SIMPLE.content();
    workflow
        .changes
        .retain(|each| !(each.from == key(OPEN) && each.to == key(DONE)));
    assert_eq!(
        judge(
            &workflow,
            &key(OPEN),
            &key(DONE),
            &anyone(),
            &Facts::default()
        ),
        refused(Why::NoSuchChange, None)
    );
}

#[test]
fn wf_03_who_can_do_it_allows_only_the_named_roles_and_people_in_the_judge() {
    let mut source = SequentialIds::default();
    let lead = RoleId::generate(&mut source).unwrap();
    let member = RoleId::generate(&mut source).unwrap();
    let other = RoleId::generate(&mut source).unwrap();
    let testers = GroupId::generate(&mut source).unwrap();
    let others = GroupId::generate(&mut source).unwrap();
    let workflow = simple_with(StatusRules {
        who: WhoCan {
            roles: vec![lead],
            groups: vec![testers],
            assigned: true,
            reporter: false,
        },
        ..StatusRules::default()
    });
    let facts = Facts::default();
    let by = |changer: Changer<'_>| judged(&workflow, &changer, &facts);
    // A named role, a named group, and the person the issue is assigned to.
    assert_eq!(
        by(Changer {
            roles: &[member, lead],
            ..anyone()
        }),
        Verdict::Allowed
    );
    assert_eq!(
        by(Changer {
            groups: &[testers],
            ..anyone()
        }),
        Verdict::Allowed
    );
    assert_eq!(
        by(Changer {
            assigned: true,
            ..anyone()
        }),
        Verdict::Allowed
    );
    // Somebody else with another role, in another group, who only reported the issue.
    assert_eq!(
        by(Changer {
            roles: &[member, other],
            groups: &[others],
            reporter: true,
            ..anyone()
        }),
        refused(Why::NotNamed, None)
    );
    // The person who reported it, when the change names them.
    let for_reporters = simple_with(StatusRules {
        who: WhoCan {
            reporter: true,
            ..WhoCan::default()
        },
        ..StatusRules::default()
    });
    let reporter = Changer {
        reporter: true,
        ..anyone()
    };
    assert_eq!(judged(&for_reporters, &reporter, &facts), Verdict::Allowed);
    assert_eq!(
        judged(&for_reporters, &anyone(), &facts),
        refused(Why::NotNamed, None)
    );
}

#[test]
fn wf_03_a_rule_never_gives_a_right_change_issues_does_not_in_the_judge() {
    // The change names everybody, or the reporter and the assigned: still, a person who may not
    // change the status of the issue is turned away.
    for who in [
        WhoCan::default(),
        WhoCan {
            reporter: true,
            assigned: true,
            ..WhoCan::default()
        },
    ] {
        let workflow = simple_with(StatusRules {
            who,
            ..StatusRules::default()
        });
        let changer = Changer {
            may_change_status: false,
            assigned: true,
            reporter: true,
            ..anyone()
        };
        assert_eq!(
            judged(&workflow, &changer, &Facts::default()),
            refused(Why::NotForYou, None)
        );
    }
}

#[test]
fn wf_03_only_if_refuses_with_a_sentence_in_the_judge() {
    let field = customer();
    let workflow = simple_with(StatusRules {
        only_if: vec![Condition {
            field,
            test: Test::Filled,
        }],
        ..StatusRules::default()
    });
    let empty = Facts::default().can(field, FieldLevel::CanSee);
    assert_eq!(
        judged(&workflow, &anyone(), &empty),
        refused(Why::OnlyIf, Some(field))
    );
    let filled = Facts::default()
        .can(field, FieldLevel::CanSee)
        .with_value(field);
    assert_eq!(judged(&workflow, &anyone(), &filled), Verdict::Allowed);
    // The other test: only if the field has no value.
    let when_empty = simple_with(StatusRules {
        only_if: vec![Condition {
            field,
            test: Test::Empty,
        }],
        ..StatusRules::default()
    });
    assert_eq!(judged(&when_empty, &anyone(), &empty), Verdict::Allowed);
    assert_eq!(
        judged(&when_empty, &anyone(), &filled),
        refused(Why::OnlyIf, Some(field))
    );
}

#[test]
fn wf_04_must_be_filled_in_names_the_missing_field_in_the_judge() {
    let field = customer();
    let workflow = simple_with(StatusRules {
        must_fill: vec![field, description()],
        ..StatusRules::default()
    });
    let facts = Facts::default()
        .can(field, FieldLevel::CanChange)
        .can(description(), FieldLevel::CanChange);
    // Both are empty: the change needs both, in the order the rule lists them.
    assert_eq!(
        judged(&workflow, &anyone(), &facts),
        Verdict::Needs(vec![field, description()])
    );
    // One has a value after the change: only the other is needed.
    let one = facts.with_value(description());
    assert_eq!(
        judged(&workflow, &anyone(), &one),
        Verdict::Needs(vec![field])
    );
    let both = one.with_value(field);
    assert_eq!(judged(&workflow, &anyone(), &both), Verdict::Allowed);
}

#[test]
fn wf_04_ask_for_needs_the_fields_the_request_does_not_carry() {
    let field = customer();
    let workflow = simple_with(StatusRules {
        ask_for: vec![field],
        ..StatusRules::default()
    });
    let facts = Facts::default()
        .can(field, FieldLevel::CanChange)
        .with_value(field);
    // A field that has a value is still asked for, until the request carries it.
    assert_eq!(
        judged(&workflow, &anyone(), &facts),
        Verdict::Needs(vec![field])
    );
    let carried = facts.carrying(field);
    assert_eq!(judged(&workflow, &anyone(), &carried), Verdict::Allowed);
}

#[test]
fn wf_04_ask_for_never_asks_for_a_hidden_field_in_the_judge() {
    let hidden = customer();
    for rules in [
        StatusRules {
            ask_for: vec![hidden],
            ..StatusRules::default()
        },
        StatusRules {
            must_fill: vec![hidden],
            ..StatusRules::default()
        },
        StatusRules {
            only_if: vec![Condition {
                field: hidden,
                test: Test::Filled,
            }],
            ..StatusRules::default()
        },
    ] {
        let workflow = simple_with(rules);
        // Whether the hidden field has a value or not, the answer is the same and names no field.
        for facts in [Facts::default(), Facts::default().with_value(hidden)] {
            assert_eq!(
                judged(&workflow, &anyone(), &facts),
                refused(Why::CannotFill, None)
            );
        }
    }
}

#[test]
fn wf_04_a_field_the_person_can_see_but_not_change_cannot_be_asked_for() {
    let field = customer();
    let workflow = simple_with(StatusRules {
        ask_for: vec![field],
        ..StatusRules::default()
    });
    let facts = Facts::default().can(field, FieldLevel::CanSee);
    // The person sees the field, so the sentence may name it.
    assert_eq!(
        judged(&workflow, &anyone(), &facts),
        refused(Why::CannotFill, Some(field))
    );
}

#[test]
fn wf_03_an_issue_outside_its_workflow_may_go_only_to_the_starting_status() {
    let workflow = SIMPLE.content();
    let gone = key("testing");
    let facts = Facts::default();
    // The starting status is the only way in; the rules of the other changes are not skipped.
    assert_eq!(
        judge(&workflow, &gone, &key(OPEN), &anyone(), &facts),
        Verdict::Allowed
    );
    for target in [IN_PROGRESS, DONE, "elsewhere"] {
        assert_eq!(
            judge(&workflow, &gone, &key(target), &anyone(), &facts),
            refused(Why::NoSuchChange, None),
            "{target}"
        );
    }
    // It is still not a right of its own.
    let nobody = Changer {
        may_change_status: false,
        ..anyone()
    };
    assert_eq!(
        judge(&workflow, &gone, &key(OPEN), &nobody, &facts),
        refused(Why::NotForYou, None)
    );
}

#[test]
fn wf_03_every_reason_has_its_own_error_code_and_message() {
    let mut codes = BTreeSet::new();
    for why in Why::ALL {
        assert!(codes.insert(why.error().code()));
        assert!(
            !crate::message::Message::from(why.error().message())
                .render()
                .is_empty()
        );
    }
}

mod properties {
    use proptest::prelude::*;

    use super::*;

    /// A few fields to name in rules: two built-in and two of the organization.
    fn pool() -> [FieldKey; 4] {
        let mut source = SequentialIds::default();
        [
            description(),
            FieldKey::BuiltIn(Field::AssignedTo),
            FieldKey::Own(FieldId::generate(&mut source).unwrap()),
            FieldKey::Own(FieldId::generate(&mut source).unwrap()),
        ]
    }

    fn level() -> impl Strategy<Value = FieldLevel> {
        prop_oneof![
            Just(FieldLevel::Hidden),
            Just(FieldLevel::CanSee),
            Just(FieldLevel::CanChange)
        ]
    }

    fn some_of(fields: &[FieldKey; 4], chosen: [bool; 4]) -> Vec<FieldKey> {
        fields
            .iter()
            .zip(chosen)
            .filter_map(|(field, yes)| yes.then_some(*field))
            .collect()
    }

    proptest! {
        #[test]
        fn wf_03_whatever_the_rules_are_they_give_no_right_and_name_no_hidden_field(
            levels in proptest::array::uniform4(level()),
            filled in proptest::array::uniform4(any::<bool>()),
            given in proptest::array::uniform4(any::<bool>()),
            ask_for in proptest::array::uniform4(any::<bool>()),
            must_fill in proptest::array::uniform4(any::<bool>()),
            only_if in proptest::array::uniform4(any::<bool>()),
            is_empty_test in any::<bool>(),
            who_assigned in any::<bool>(),
            who_reporter in any::<bool>(),
            may_change_status in any::<bool>(),
            assigned in any::<bool>(),
            reporter in any::<bool>(),
        ) {
            let fields = pool();
            let mut facts = Facts::default();
            for (index, field) in fields.iter().enumerate() {
                facts = facts.can(*field, levels[index]);
                if filled[index] {
                    facts = facts.with_value(*field);
                }
                if given[index] {
                    facts = facts.carrying(*field);
                }
            }
            let test = if is_empty_test { Test::Empty } else { Test::Filled };
            let workflow = simple_with(StatusRules {
                who: WhoCan {
                    assigned: who_assigned,
                    reporter: who_reporter,
                    ..WhoCan::default()
                },
                only_if: some_of(&fields, only_if)
                    .into_iter()
                    .map(|field| Condition { field, test })
                    .collect(),
                must_fill: some_of(&fields, must_fill),
                ask_for: some_of(&fields, ask_for),
            });
            let changer = Changer {
                may_change_status,
                assigned,
                reporter,
                ..anyone()
            };
            let verdict = judged(&workflow, &changer, &facts);
            // A rule never gives a right (WF-03).
            if !may_change_status {
                prop_assert!(matches!(verdict, Verdict::NotAllowed(_)), "{verdict:?}");
            }
            // A field the person may not see is never named, and none they may not change is asked
            // for (SEC-06, WF-04).
            let named: Vec<FieldKey> = match &verdict {
                Verdict::Allowed => Vec::new(),
                Verdict::Needs(needed) => {
                    for field in needed {
                        prop_assert_eq!(facts.level(*field), FieldLevel::CanChange);
                    }
                    needed.clone()
                }
                Verdict::NotAllowed(because) => because.field.into_iter().collect(),
            };
            for field in named {
                prop_assert!(facts.level(field) > FieldLevel::Hidden, "{verdict:?}");
            }
        }
    }
}
