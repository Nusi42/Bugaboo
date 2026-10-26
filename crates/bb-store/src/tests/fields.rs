//! Own fields on both databases (CF-01, CF-04, CF-07, ADR-0006, ADR-0027): every type stores and
//! returns its value, a combined choice moves every value, and a level for an issue type is found
//! before the level for all types.

use bb_app::access::PolicyStore as _;
use bb_app::fields::FieldStore;
use bb_app::issues::IssueStore as _;
use bb_app::ports::{BoxFuture, ChangeStore};
use bb_core::access::FieldLevel;
use bb_core::event::{EntityKind, RecordKind};
use bb_core::fields::FieldKey;
use bb_core::id::{EntityId, IdSource, IssueTypeId, ProjectId, RoleId};
use bb_core::issue::{IssueIcon, IssueType};
use bb_core::own_fields::FieldType;
use bb_core::own_fields_value::FieldValue;
use bb_core::project::{Project, ShortName};
use bb_core::version::FIRST_VERSION;
use bb_testing::{TestResult, each_database};
use proptest::prelude::*;
use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::{Config, TestRunner};

use super::*;
use crate::test_support::{at, definition, event, fixture, ids, world};

/// Runs `write` as a change with its event, the way a use case does.
async fn changed<T: Send>(
    storage: &AuthStorage,
    source: &mut impl IdSource,
    write: impl for<'t> FnOnce(&'t mut Transaction) -> BoxFuture<'t, Result<T, StoreError>> + Send,
) -> Result<T, StoreError> {
    ChangeStore::record_change(storage, &event(source, 5), write).await
}

/// Values of this type that the type can hold, drawn by proptest.
fn values_of(kind: FieldType, count: usize) -> Vec<FieldValue> {
    let id = prop::array::uniform16(any::<u8>());
    let strategy: BoxedStrategy<FieldValue> = match kind {
        FieldType::Text | FieldType::WebAddress => {
            "[a-zA-Z0-9äöüß ]{1,40}".prop_map(FieldValue::Text).boxed()
        }
        // Printable characters, line breaks and tabs: what a long text may hold.
        FieldType::LongText => "[ -~\n\täöü€😀]{1,300}".prop_map(FieldValue::Text).boxed(),
        FieldType::WholeNumber | FieldType::Decimal | FieldType::Duration => {
            prop_oneof![any::<i64>(), -1_000_000_i64..1_000_000]
                .prop_map(FieldValue::Number)
                .boxed()
        }
        FieldType::YesNo => any::<bool>().prop_map(FieldValue::Flag).boxed(),
        FieldType::Date => (-100_000_i64..100_000).prop_map(FieldValue::Day).boxed(),
        FieldType::DateTime => any::<i64>().prop_map(FieldValue::Moment).boxed(),
        FieldType::PickOne | FieldType::Person | FieldType::Group | FieldType::Issue => {
            id.prop_map(|bytes| FieldValue::Ids(vec![bytes])).boxed()
        }
        FieldType::PickSeveral | FieldType::People => prop::collection::vec(id, 1..4)
            .prop_map(FieldValue::Ids)
            .boxed(),
    };
    let mut runner = TestRunner::new(Config::default());
    (0..count)
        .map(|_| strategy.new_tree(&mut runner).unwrap().current())
        .collect()
}

#[tokio::test]
async fn cf_01_every_field_type_stores_and_returns_its_value() -> TestResult {
    each_database("cf_01_every_type", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let issue_kind = EntityKind::from(RecordKind::Issue);
        let mut position = 0;
        for kind in FieldType::ALL {
            position += 1;
            let made = definition(&mut source, *kind, position);
            let field = made.field.id;
            changed(storage, &mut source, move |tx| {
                Box::pin(async move { AuthStorage::insert_field(tx, &made).await })
            })
            .await?;
            for value in values_of(*kind, 12) {
                let entity = EntityId::from(IssueTypeId::generate(&mut source).unwrap());
                let (cells, kind_now) = (value.to_cells(), issue_kind.clone());
                changed(storage, &mut source, move |tx| {
                    Box::pin(async move {
                        AuthStorage::put_field_value(tx, kind_now, entity, field, &cells).await
                    })
                })
                .await?;
                let stored = storage
                    .field_values(issue_kind.clone(), vec![entity])
                    .await?;
                assert_eq!(stored.len(), 1, "{kind:?}");
                assert_eq!(
                    FieldValue::from_cells(*kind, &stored[0].cells),
                    Some(value),
                    "{kind:?}"
                );
            }
        }
        // The definitions come back with their choices in order.
        let all = storage.field_definitions().await?;
        assert_eq!(all.len(), FieldType::ALL.len());
        let pick = all
            .iter()
            .find(|found| found.field.kind == FieldType::PickOne)
            .unwrap();
        assert_eq!(pick.choices.len(), 3);
        assert_eq!(pick.choices[0].name, "Minor");
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cf_07_combining_two_choices_moves_every_value_and_keeps_one_row_for_an_item_with_both()
-> TestResult {
    each_database("cf_07_combine", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let issue_kind = EntityKind::from(RecordKind::Issue);
        let made = definition(&mut source, FieldType::PickSeveral, 1);
        let (field, minor, small, major) = (
            made.field.id,
            made.choices[0].id,
            made.choices[1].id,
            made.choices[2].id,
        );
        let stored = made.clone();
        changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::insert_field(tx, &stored).await })
        })
        .await?;
        let items: Vec<EntityId> = (0..3)
            .map(|_| EntityId::from(IssueTypeId::generate(&mut source).unwrap()))
            .collect();
        // One item holds only `small`, one holds both, one holds `major`.
        let held = [vec![small], vec![small, minor], vec![major]];
        for (item, choices) in items.iter().zip(&held) {
            let cells =
                FieldValue::Ids(choices.iter().map(|id| id.to_bytes()).collect()).to_cells();
            let (item, kind) = (*item, issue_kind.clone());
            changed(storage, &mut source, move |tx| {
                Box::pin(async move {
                    AuthStorage::put_field_value(tx, kind, item, field, &cells).await
                })
            })
            .await?;
        }
        let kind = issue_kind.clone();
        let touched = changed(storage, &mut source, move |tx| {
            Box::pin(
                async move { AuthStorage::combine_choice_values(tx, field, small, minor).await },
            )
        })
        .await?;
        let touched: Vec<EntityId> = touched.into_iter().map(|(_, item)| item).collect();
        assert_eq!(touched.len(), 2);
        assert!(touched.contains(&items[0]) && touched.contains(&items[1]));
        let values = storage.field_values(kind, items.clone()).await?;
        let ids_of = |item: EntityId| -> Vec<[u8; 16]> {
            let found = values.iter().find(|found| found.entity == item).unwrap();
            FieldValue::from_cells(FieldType::PickSeveral, &found.cells)
                .unwrap()
                .ids()
                .to_vec()
        };
        assert_eq!(ids_of(items[0]), vec![minor.to_bytes()]);
        assert_eq!(ids_of(items[1]), vec![minor.to_bytes()]);
        assert_eq!(ids_of(items[2]), vec![major.to_bytes()]);
        // Nothing holds the combined choice any more.
        assert!(storage.items_with_choice(field, small).await?.is_empty());
        Ok(())
    })
    .await
}

/// How many items hold a field in the test of chunks, and how many of them hold the first choice.
const ITEMS: usize = 33_000;
const MINOR: usize = ITEMS - 200;

#[tokio::test]
async fn cf_07_a_choice_held_by_thousands_of_items_is_combined_in_chunks_and_each_item_counts_up()
-> TestResult {
    each_database("cf_07_combine_in_chunks", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        // More items than SQLite takes in one statement (32,766 values): every read and write has to
        // be in chunks, and a choice held by a hundred thousand items was a failure before.
        let world = world(storage, &mut source, i64::try_from(ITEMS)?).await;
        let made = definition(&mut source, FieldType::PickOne, 1);
        let (field, minor, major) = (made.field.id, made.choices[0].id, made.choices[1].id);
        let stored = made.clone();
        changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::insert_field(tx, &stored).await })
        })
        .await?;
        let issue_kind = EntityKind::from(RecordKind::Issue);
        let items: Vec<EntityId> = world
            .issues
            .iter()
            .map(|issue| EntityId::from(issue.id))
            .collect();
        // Most hold Minor, the last 200 hold Major.
        let (kind, held) = (issue_kind.clone(), items.clone());
        changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                for (at, item) in held.iter().enumerate() {
                    let choice = if at < MINOR { minor } else { major };
                    let cells = FieldValue::Ids(vec![choice.to_bytes()]).to_cells();
                    AuthStorage::put_field_value(tx, kind.clone(), *item, field, &cells).await?;
                }
                Ok(())
            })
        })
        .await?;

        // All the values are read back, though they need three statements.
        let values = storage
            .field_values(issue_kind.clone(), items.clone())
            .await?;
        assert_eq!(values.len(), ITEMS);

        let touched = changed(storage, &mut source, move |tx| {
            Box::pin(
                async move { AuthStorage::combine_choice_values(tx, field, minor, major).await },
            )
        })
        .await?;
        assert_eq!(touched.len(), MINOR);
        // Counting the versions up goes in the order given and says the version each had.
        let mut counted = touched.clone();
        counted.push((EntityKind::from(RecordKind::Field), EntityId::from(field)));
        let versions = changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::touch_items(tx, &counted, at(9)).await })
        })
        .await?;
        assert_eq!(versions.len(), MINOR + 1);
        assert!(
            versions[..1_100]
                .iter()
                .all(|version| *version == Some(FIRST_VERSION))
        );
        // A kind of item that has no version says so, and the others are not disturbed by it.
        assert_eq!(versions[MINOR], None);
        // The first, the ones on both sides of the edges of chunks and the last that was touched.
        for place in [0, 1, 499, 500, 501, 999, 1_000, 16_383, MINOR - 1] {
            let touched = storage.issue(world.issues[place].id).await?.unwrap();
            assert_eq!(
                (touched.version, touched.updated_at),
                (FIRST_VERSION + 1, at(9)),
                "the issue at {place}"
            );
        }
        let last = storage.issue(world.issues[ITEMS - 1].id).await?.unwrap();
        assert_eq!(last.version, FIRST_VERSION);
        // Every item holds Major now.
        let values = storage.field_values(issue_kind, items).await?;
        assert!(values.iter().all(|found| {
            FieldValue::from_cells(FieldType::PickOne, &found.cells)
                .is_some_and(|value| value.ids() == [major.to_bytes()])
        }));
        Ok(())
    })
    .await
}

#[tokio::test]
async fn adr_0027_a_level_for_an_issue_type_is_stored_apart_and_found_with_the_rules() -> TestResult
{
    each_database("adr_0027_typed_levels", async |db| {
        let fixture = fixture(db).await?;
        let storage = &fixture.storage;
        let mut source = ids();
        let made = definition(&mut source, FieldType::Text, 1);
        let key = FieldKey::Own(made.field.id);
        let stored = made.clone();
        changed(storage, &mut source, move |tx| {
            Box::pin(async move { AuthStorage::insert_field(tx, &stored).await })
        })
        .await?;
        // A role with a level of its own for the field, and one for a type.
        let role = RoleId::generate(&mut source).unwrap();
        let bug = IssueTypeId::generate(&mut source).unwrap();
        let project = ProjectId::generate(&mut source).unwrap();
        let org_project = Project {
            id: project,
            parent: None,
            short_name: ShortName::new("ACME").unwrap(),
            name: "Acme".to_owned(),
            version: FIRST_VERSION,
        };
        let role_row = bb_core::access::Role {
            id: role,
            name: "Reader".to_owned(),
            starting: None,
            position: 0,
            two_step: false,
            version: FIRST_VERSION,
        };
        changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::insert_project(tx, &org_project).await?;
                AuthStorage::insert_role(tx, &role_row, &[], &[(key, FieldLevel::CanSee)], &[])
                    .await?;
                AuthStorage::insert_issue_type(
                    tx,
                    &IssueType {
                        project: None,
                        id: bug,
                        name: "Bug".to_owned(),
                        icon: IssueIcon::Bug,
                        position: 0,
                        retired_at: None,
                        version: FIRST_VERSION,
                    },
                )
                .await?;
                AuthStorage::put_level(tx, role, None, Some(bug), key, Some(FieldLevel::Hidden))
                    .await?;
                AuthStorage::put_level(
                    tx,
                    role,
                    Some(project),
                    Some(bug),
                    key,
                    Some(FieldLevel::CanChange),
                )
                .await
            })
        })
        .await?;
        // The level for all types is untouched by the levels for the type.
        let all: Vec<(FieldKey, FieldLevel)> = storage.levels(role, None, None).await?;
        assert_eq!(all, vec![(key, FieldLevel::CanSee)]);
        let typed: Vec<(FieldKey, FieldLevel)> = storage.levels(role, None, Some(bug)).await?;
        assert_eq!(typed, vec![(key, FieldLevel::Hidden)]);
        let in_project: Vec<(FieldKey, FieldLevel)> =
            storage.levels(role, Some(project), Some(bug)).await?;
        assert_eq!(in_project, vec![(key, FieldLevel::CanChange)]);
        // The rules of a place hold the typed rows and know the own field.
        let found = storage
            .level_rules(vec![role], Some(project))
            .await?
            .unwrap();
        assert_eq!(found.rules.typed.len(), 2);
        assert_eq!(found.rules.own_fields, vec![made.field.id]);
        // Taking a level away.
        changed(storage, &mut source, move |tx| {
            Box::pin(async move {
                AuthStorage::put_level::<FieldKey>(tx, role, None, Some(bug), key, None).await
            })
        })
        .await?;
        let typed: Vec<(FieldKey, FieldLevel)> = storage.levels(role, None, Some(bug)).await?;
        assert!(typed.is_empty());
        Ok(())
    })
    .await
}
