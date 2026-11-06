//! The number of a choice (TIME-13, CF-08): an administrator sets it, it is unique within the
//! field, the choices of a field with numbers are listed by the number, and a field without keeps
//! the order the administrator set.

use bb_core::ErrorCode;
use bb_core::own_fields::{Edit, FieldType};

use super::scenarios::{text, token};
use super::scenarios_fields::a_field;
use super::world::{Outcome, World};
use crate::auth::AppError;
use crate::fields::{AddChoice, ChangeChoice, FieldView, add_choice, change_choice};

fn adding(world: &World, field: &FieldView, name: &str, number: Option<&str>) -> AddChoice {
    AddChoice {
        session_token: token(world),
        field: field.id,
        version: field.version,
        name: name.to_owned(),
        number: number.map(str::to_owned),
        color: None,
        icon: None,
    }
}

fn numbering(world: &World, field: &FieldView, at: usize, number: Option<&str>) -> ChangeChoice {
    ChangeChoice {
        session_token: token(world),
        field: field.id,
        choice: field.choices[at].id,
        version: field.version,
        name: None,
        // A number is set, or taken away when there is none.
        number: number.map_or(Edit::Clear, |text| Edit::Set(text.to_owned())),
        color: None,
        icon: Edit::Keep,
        retired: None,
    }
}

fn shown(field: &FieldView) -> Vec<&str> {
    field
        .choices
        .iter()
        .map(|choice| choice.shown_as.as_str())
        .collect()
}

#[tokio::test]
async fn time_13_choices_with_numbers_are_listed_by_number_and_written_with_it() -> Outcome {
    let world = World::new().await?;
    let field = a_field(&world, "Budget line", FieldType::PickOne, &[]).await?;
    let sales = add_choice(&world.ctx(), adding(&world, &field, "Sales", Some("5200")))
        .await
        .map_err(text)?
        .into_value();
    let both = add_choice(
        &world.ctx(),
        adding(&world, &sales, "Product development", Some("4100")),
    )
    .await
    .map_err(text)?
    .into_value();
    let plain = add_choice(&world.ctx(), adding(&world, &both, "Overhead", None))
        .await
        .map_err(text)?
        .into_value();
    assert_eq!(
        shown(&plain),
        ["4100 Product development", "5200 Sales", "Overhead"]
    );
    assert_eq!(plain.choices[0].number, Some("4100".to_owned()));
    assert_eq!(plain.choices[0].name, "Product development");
    Ok(())
}

#[tokio::test]
async fn time_13_a_number_used_twice_is_told_and_not_saved() -> Outcome {
    let world = World::new().await?;
    let field = a_field(&world, "Budget line", FieldType::PickOne, &[]).await?;
    let one = add_choice(&world.ctx(), adding(&world, &field, "Sales", Some("5200")))
        .await
        .map_err(text)?
        .into_value();
    let twice = add_choice(
        &world.ctx(),
        adding(&world, &one, "Marketing", Some("5200")),
    )
    .await;
    assert!(
        matches!(twice, Err(AppError::Refused(ErrorCode::ChoiceNumberTaken))),
        "{twice:?}"
    );
    let two = add_choice(&world.ctx(), adding(&world, &one, "Marketing", None))
        .await
        .map_err(text)?
        .into_value();
    let renumbered = change_choice(&world.ctx(), numbering(&world, &two, 1, Some("5200"))).await;
    assert!(
        matches!(
            renumbered,
            Err(AppError::Refused(ErrorCode::ChoiceNumberTaken))
        ),
        "{renumbered:?}"
    );
    // A choice keeps its own number when it is sent again, and a number may be taken away.
    let kept = change_choice(&world.ctx(), numbering(&world, &two, 0, Some("5200")))
        .await
        .map_err(text)?
        .into_value();
    assert_eq!(shown(&kept), ["5200 Sales", "Marketing"]);
    let cleared = change_choice(&world.ctx(), numbering(&world, &kept, 0, None))
        .await
        .map_err(text)?
        .into_value();
    assert_eq!(shown(&cleared), ["Sales", "Marketing"]);
    // A number outside the whole numbers a choice may have is no number.
    let negative = add_choice(&world.ctx(), adding(&world, &cleared, "Legal", Some("-1"))).await;
    assert!(
        matches!(negative, Err(AppError::Refused(ErrorCode::InvalidRequest))),
        "{negative:?}"
    );
    Ok(())
}

#[tokio::test]
async fn time_13_a_field_without_numbers_keeps_the_order_of_the_administrator() -> Outcome {
    let world = World::new().await?;
    let field = a_field(
        &world,
        "Kind of work",
        FieldType::PickOne,
        &["Review", "Develop", "Admin"],
    )
    .await?;
    assert_eq!(shown(&field), ["Review", "Develop", "Admin"]);
    let numbered = change_choice(&world.ctx(), numbering(&world, &field, 2, Some("10")))
        .await
        .map_err(text)?
        .into_value();
    assert_eq!(shown(&numbered), ["10 Admin", "Review", "Develop"]);
    Ok(())
}

#[tokio::test]
async fn time_13_a_number_keeps_its_digits_and_choices_sort_by_their_value() -> Outcome {
    let world = World::new().await?;
    let field = a_field(&world, "Budget line", FieldType::PickOne, &[]).await?;
    let mut now = field;
    for (name, number) in [
        ("Ten", "10"),
        ("Padded", "0410"),
        ("Nine", "9"),
        ("Plain", "410"),
    ] {
        now = add_choice(&world.ctx(), adding(&world, &now, name, Some(number)))
            .await
            .map_err(text)?
            .into_value();
    }
    // 9 before 10; 0410 and 410 are different numbers and come in one fixed order.
    assert_eq!(
        shown(&now),
        ["9 Nine", "10 Ten", "0410 Padded", "410 Plain"]
    );
    // Letters, signs and more than nine digits are no number.
    for bad in ["K-100", "41 0", "1234567890", ""] {
        let refused = add_choice(&world.ctx(), adding(&world, &now, "Other", Some(bad))).await;
        assert!(
            matches!(refused, Err(AppError::Refused(ErrorCode::InvalidRequest))),
            "{bad}: {refused:?}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn time_13_no_two_choices_are_found_by_the_same_name_number_or_both_together() -> Outcome {
    let world = World::new().await?;
    let field = a_field(&world, "Budget line", FieldType::PickOne, &[]).await?;
    let sales = add_choice(&world.ctx(), adding(&world, &field, "Sales", Some("5200")))
        .await
        .map_err(text)?
        .into_value();
    // A name that is the number of another choice, or the two written together, is taken.
    for name in ["5200", "5200 Sales", "5200   sales"] {
        let refused = add_choice(&world.ctx(), adding(&world, &sales, name, None)).await;
        assert!(
            matches!(refused, Err(AppError::Refused(ErrorCode::NameTaken))),
            "{name}: {refused:?}"
        );
    }
    // A number that is the name of another choice is taken as a number.
    let named = add_choice(&world.ctx(), adding(&world, &sales, "4100", None))
        .await
        .map_err(text)?
        .into_value();
    let refused = add_choice(
        &world.ctx(),
        adding(&world, &named, "Product", Some("4100")),
    )
    .await;
    assert!(
        matches!(
            refused,
            Err(AppError::Refused(ErrorCode::ChoiceNumberTaken))
        ),
        "{refused:?}"
    );
    Ok(())
}

#[tokio::test]
async fn time_13_a_clash_of_keys_is_refused_in_both_directions_by_the_member_that_changed()
-> Outcome {
    let world = World::new().await?;
    let field = a_field(&world, "Budget line", FieldType::PickOne, &[]).await?;
    // A choice named like the number and the name together, without a number ...
    let named = add_choice(
        &world.ctx(),
        adding(&world, &field, "4100 Product development", None),
    )
    .await
    .map_err(text)?
    .into_value();
    // ... stops a new choice with that name and number, which is found by the same words.
    let refused = add_choice(
        &world.ctx(),
        adding(&world, &named, "Product development", Some("4100")),
    )
    .await;
    assert!(
        matches!(
            refused,
            Err(AppError::Refused(ErrorCode::ChoiceNumberTaken))
        ),
        "{refused:?}"
    );
    // The other direction: the choice with the number first, then the one named like both.
    let other = a_field(&world, "Cost line", FieldType::PickOne, &[]).await?;
    let numbered = add_choice(
        &world.ctx(),
        adding(&world, &other, "Product development", Some("4100")),
    )
    .await
    .map_err(text)?
    .into_value();
    let refused = add_choice(
        &world.ctx(),
        adding(&world, &numbered, "4100 Product development", None),
    )
    .await;
    assert!(
        matches!(refused, Err(AppError::Refused(ErrorCode::NameTaken))),
        "{refused:?}"
    );
    // A change of the number alone into a clash is told as the number, one of the name alone as
    // the name.
    let plain = add_choice(
        &world.ctx(),
        adding(&world, &named, "Product development", None),
    )
    .await
    .map_err(text)?
    .into_value();
    let renumbered = change_choice(&world.ctx(), numbering(&world, &plain, 1, Some("4100"))).await;
    assert!(
        matches!(
            renumbered,
            Err(AppError::Refused(ErrorCode::ChoiceNumberTaken))
        ),
        "{renumbered:?}"
    );
    Ok(())
}
