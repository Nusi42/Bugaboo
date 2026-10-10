//! Every command use case appends its domain event (ADR-0007, M1-F18), run against the in-memory
//! store.
//!
//! ARCH-EVENTS reads the code and finds every command use case: an `async fn` of `bb-app` with
//! the context whose input type does not end in `Query`, and its body must call `record_change`.
//! This test runs each of them: one scenario per use case (`scenarios.rs`), and the table below
//! must name every command of `AuthApi` and the two the server calls itself, and nothing else.
//! The check also finds a command that is missing from the table (the names in it are strings of
//! this file).

use std::future::Future;
use std::pin::Pin;

use super::scenarios::{self as s, Added};
use super::scenarios_issues as si;
use super::world::{Outcome, World};
use crate::auth::USE_CASES;

/// A scenario: starts from a world that has its first administrator, and says how many events the
/// call under test appended.
type Scenario = for<'a> fn(&'a World) -> Pin<Box<dyn Future<Output = Added> + 'a>>;

/// The use cases the server calls itself, at every start, and not through `AuthApi`.
const SYSTEM_USE_CASES: [&str; 3] = [
    "issue_setup_code",
    "make_starting_roles",
    "make_starting_issue_types",
];

/// The table: the name of the use case and its scenario. One line per command use case.
macro_rules! table {
    ($($name:literal => $scenario:path),+ $(,)?) => {
        const TABLE: &[(&str, Scenario)] = &[
            $(($name, |world| Box::pin($scenario(world)))),+
        ];
    };
}

table! {
    "sign_in" => s::sign_in_scenario,
    "second_step" => s::second_step_scenario,
    "passkey_second_step" => s::passkey_second_step_scenario,
    "passkey_sign_in" => s::passkey_sign_in_scenario,
    "sign_out" => s::sign_out_scenario,
    "sign_out_everywhere" => s::sign_out_everywhere_scenario,
    "end_session" => s::end_session_scenario,
    "change_password" => s::change_password_scenario,
    "start_code_app" => s::start_code_app_scenario,
    "confirm_code_app" => s::confirm_code_app_scenario,
    "remove_code_app" => s::remove_code_app_scenario,
    "renew_backup_codes" => s::renew_backup_codes_scenario,
    "add_passkey" => s::add_passkey_scenario,
    "remove_passkey" => s::remove_passkey_scenario,
    "create_invitation" => s::create_invitation_scenario,
    "revoke_invitation" => s::revoke_invitation_scenario,
    "accept_invitation" => s::accept_invitation_scenario,
    "sign_up" => s::sign_up_scenario,
    "change_sign_up_policy" => s::change_sign_up_policy_scenario,
    "complete_setup" => s::complete_setup_scenario,
    "issue_setup_code" => s::issue_setup_code_scenario,
    "make_starting_roles" => s::make_starting_roles_scenario,
    "make_group" => s::make_group_scenario,
    "rename_group" => s::rename_group_scenario,
    "delete_group" => s::delete_group_scenario,
    "add_group_member" => s::add_group_member_scenario,
    "remove_group_member" => s::remove_group_member_scenario,
    "make_role" => s::make_role_scenario,
    "copy_role" => s::copy_role_scenario,
    "change_role" => s::change_role_scenario,
    "delete_role" => s::delete_role_scenario,
    "reset_role" => s::reset_role_scenario,
    "assign_role" => s::assign_role_scenario,
    "remove_assignment" => s::remove_assignment_scenario,
    "set_level" => s::set_field_level_scenario,
    "set_level" => s::set_people_level_scenario,
    "make_project" => s::make_project_scenario,
    "change_setting" => s::change_setting_scenario,
    "reset_setting" => s::reset_setting_scenario,
    "lock_setting" => s::lock_setting_scenario,
    "make_access_key" => s::make_access_key_scenario,
    "delete_access_key" => s::delete_access_key_scenario,
    "change_project" => s::change_project_scenario,
    "move_project" => s::move_project_scenario,
    "ask_to_finish" => s::ask_to_finish_scenario,
    "edit_anyway" => s::edit_anyway_scenario,
    "release_stuck_lock" => s::release_stuck_lock_scenario,
    "make_issue" => si::make_issue_scenario,
    "change_issue" => si::change_issue_scenario,
    "delete_issue" => si::delete_issue_scenario,
    "restore_issue" => si::restore_issue_scenario,
    "make_issue_type" => si::make_issue_type_scenario,
    "change_issue_type" => si::change_issue_type_scenario,
    "make_starting_issue_types" => si::make_starting_issue_types_scenario,
}

#[test]
fn adr_0007_the_table_names_every_command_use_case_and_nothing_else() {
    let mut commands: Vec<&str> = USE_CASES
        .iter()
        .filter(|(_, input)| !input.ends_with("Query"))
        .map(|(name, _)| *name)
        .chain(SYSTEM_USE_CASES)
        .collect();
    commands.sort_unstable();
    let mut named: Vec<&str> = TABLE.iter().map(|(name, _)| *name).collect();
    named.sort_unstable();
    assert_eq!(
        named, commands,
        "the scenarios and the command use cases differ"
    );
}

#[tokio::test]
async fn adr_0007_every_command_use_case_appends_its_event() -> Outcome {
    for (name, scenario) in TABLE {
        let world = World::new().await?;
        let added = scenario(&world)
            .await
            .map_err(|error| format!("{name}: {error}"))?;
        assert!(added >= 1, "{name} appended no event");
        // A change is one event; only the start of the program may make several changes at once.
        if *name != "make_starting_roles" {
            assert_eq!(added, 1, "{name} appended {added} events");
        }
    }
    Ok(())
}
