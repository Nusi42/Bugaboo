//! TEST-WORKING-TIME: the tests of the working-time rules, of the person's day and of the records
//! of work days exist (TIME-08, TIME-09, TIME-10, ADR-0035, decisions 5 to 7).
//!
//! Working hours are what German law asks every employer to record, so the guarantees are held by
//! tests that cannot be deleted unseen: every rule of the preset *Germany* warns and never refuses,
//! no rules give no warning, the person's day holds across midnight, across the changes of
//! daylight-saving time, in a zone east of UTC+12 and for a person who changed zone (property
//! tests in `bb-core`), and the records of work days keep what really happened: a day left open
//! stays open until the person says when it ended, a past day changes only with a reason and keeps
//! the old times, the person is told of a change by somebody else, and old days go only when an
//! administrator says so. The records run on both databases. This check fails when a named test is
//! missing or switched off, in the ways the one implementation knows (`required_tests`).

use super::required_tests::{Required, crate_sources, evaluate};
use super::{Check, Context, Finding};

const RULES: &str = "crates/bb-core/src/working_time/tests.rs";
const SERVER: &str = "crates/bb-server/src/tests/sign_in/working_hours_http.rs";

/// What the work package and ADR-0035 name as proof, and where each test lives.
pub(super) const REQUIRED: [Required; 22] = [
    Required::anywhere("time_09_ten_hours_and_a_minute_warns_and_is_still_recorded").within(RULES),
    Required::anywhere("time_09_breaks_under_fifteen_minutes_do_not_count").within(RULES),
    Required::anywhere("time_09_six_hours_in_a_row_without_a_break_warns").within(RULES),
    Required::anywhere("time_09_eleven_hours_of_rest_are_checked_against_the_day_before")
        .within(RULES),
    Required::anywhere("time_09_no_rules_gives_no_warning").within(RULES),
    Required::anywhere(
        "time_08_a_day_across_midnight_belongs_to_the_date_it_starts_on_and_counts_every_minute",
    )
    .within(RULES),
    Required::anywhere("time_08_a_day_across_the_daylight_saving_changes_counts_the_time_that_passed")
        .within(RULES),
    Required::anywhere("time_08_a_zone_east_of_utc_plus_twelve_starts_its_day_first").within(RULES),
    Required::anywhere(
        "time_09_a_person_who_changed_zone_between_two_days_has_the_rest_of_the_moments",
    )
    .within(RULES),
    Required::both("time_08_start_break_end_form_one_work_day_with_its_totals").within(SERVER),
    Required::both("time_08_two_starts_at_the_same_moment_keep_one_work_day").within(SERVER),
    Required::both("time_12_an_access_key_never_reaches_its_owners_working_hours").within(SERVER),
    Required::both("priv_09_working_hours_are_in_no_search_list_or_order").within(SERVER),
    Required::both("time_08_a_day_left_open_must_be_ended_before_the_next_starts").within(SERVER),
    Required::both("time_10_a_past_day_changes_only_with_a_reason_and_keeps_the_old_value")
        .within(SERVER),
    Required::both("time_10_a_change_by_someone_else_tells_the_person").within(SERVER),
    Required::both("priv_09_old_working_hours_are_deleted_only_when_an_administrator_says_so")
        .within(SERVER),
    Required::both(
        "time_12_without_the_permission_another_persons_days_answer_like_a_person_that_does_not_exist",
    )
    .within(SERVER),
    Required::both("time_12_totals_only_answers_week_totals_and_nothing_else").within(SERVER),
    Required::both("time_12_an_administrator_does_not_see_working_hours_until_given_hr")
        .within(SERVER),
    Required::both("priv_09_every_change_of_who_sees_working_hours_is_in_the_security_log")
        .within(SERVER),
    Required::anywhere(
        "time_10_a_removed_persons_work_days_keep_the_name_for_those_who_may_see_working_hours",
    )
    .within("crates/bb-policy/tests/engine/people.rs"),
];

pub(crate) struct TestWorkingTime;

impl Check for TestWorkingTime {
    fn id(&self) -> &'static str {
        "TEST-WORKING-TIME"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(
            &crate_sources(context.repo)?,
            &REQUIRED,
            "The working-time rules and the records of work days (ADR-0035) are held by it;",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::super::required_tests::examples;
    use super::*;

    #[test]
    fn good_example_the_tests_of_this_repository_are_all_there() {
        examples::this_repository_has_every_test(&REQUIRED);
    }

    #[test]
    fn bad_example_a_rule_without_its_test_or_a_record_on_one_database_fails() {
        let name = REQUIRED[0].name;
        let rule = [Required::anywhere(name).within(RULES)];
        let root = (
            "crates/bb-core/src/lib.rs".to_owned(),
            "mod working_time;\n".to_owned(),
        );
        let module = (
            "crates/bb-core/src/working_time/mod.rs".to_owned(),
            "#[cfg(test)]\nmod tests;\n".to_owned(),
        );
        let test = |text: &str| (RULES.to_owned(), text.to_owned());
        let live = format!("#[test]\nfn {name}() {{}}\n");
        let all = |text: &str| vec![root.clone(), module.clone(), test(text)];
        assert!(evaluate(&all(&live), &rule, "").is_empty());
        assert_eq!(evaluate(&all(""), &rule, "").len(), 1);
        let ignored = live.replace("#[test]", "#[test]\n#[ignore]");
        assert_eq!(evaluate(&all(&ignored), &rule, "").len(), 1);
        // A record of work days that runs on one database only does not count.
        let both = [Required::both(REQUIRED[9].name).within(SERVER)];
        let server = |text: &str| {
            vec![
                (
                    "crates/bb-server/src/lib.rs".to_owned(),
                    "mod tests;\n".to_owned(),
                ),
                (
                    "crates/bb-server/src/tests/mod.rs".to_owned(),
                    "mod sign_in;\n".to_owned(),
                ),
                (
                    "crates/bb-server/src/tests/sign_in/mod.rs".to_owned(),
                    "mod working_hours_http;\n".to_owned(),
                ),
                (SERVER.to_owned(), text.to_owned()),
            ]
        };
        let one = format!(
            "#[tokio::test]\nasync fn {}() {{ sqlite_only(); }}\n",
            REQUIRED[9].name
        );
        assert_eq!(evaluate(&server(&one), &both, "").len(), 1);
        let two = one.replace("sqlite_only()", "each_database(\"x\")");
        assert!(evaluate(&server(&two), &both, "").is_empty());
    }
}
