//! The list of every migration this program knows, by number and name: the one place a new
//! migration is added (ADR-0003, DATA-MIGRATIONS).

use super::Definition;

/// Every migration, in order. A new one is added at the end and never changed afterwards.
pub(super) const ALL: &[Definition] = &[
    Definition {
        version: 1,
        name: "initial",
        steps: super::m0001_initial::steps,
    },
    Definition {
        version: 2,
        name: "sign_in",
        steps: || Ok(super::m0002_sign_in::steps()),
    },
    Definition {
        version: 3,
        name: "policy",
        steps: super::m0003_policy::steps,
    },
    Definition {
        version: 4,
        name: "settings",
        steps: super::m0004_settings::steps,
    },
    Definition {
        version: 5,
        name: "projects",
        steps: || Ok(super::m0005_projects::steps()),
    },
    Definition {
        version: 6,
        name: "access_keys",
        steps: || Ok(super::m0006_access_keys::steps()),
    },
    Definition {
        version: 7,
        name: "issues",
        steps: || Ok(super::m0007_issues::steps()),
    },
    Definition {
        version: 8,
        name: "editing",
        steps: || Ok(super::m0008_editing::steps()),
    },
    Definition {
        version: 9,
        name: "fields",
        steps: || Ok(super::m0009_fields::steps()),
    },
    Definition {
        version: 10,
        name: "relations",
        steps: || Ok(super::m0010_relations::steps()),
    },
    Definition {
        version: 11,
        name: "tags",
        steps: || Ok(super::m0011_tags::steps()),
    },
    Definition {
        version: 12,
        name: "layouts",
        steps: || Ok(super::m0012_layouts::steps()),
    },
    Definition {
        version: 13,
        name: "job_kinds",
        steps: || Ok(super::m0013_job_kinds::steps()),
    },
    Definition {
        version: 14,
        name: "names",
        steps: || Ok(super::m0014_names::steps()),
    },
    Definition {
        version: 15,
        name: "workflows",
        steps: || Ok(super::m0015_workflows::steps()),
    },
    Definition {
        version: 16,
        name: "files",
        steps: || Ok(super::m0016_files::steps()),
    },
    Definition {
        version: 17,
        name: "notifications",
        steps: || Ok(super::m0017_notifications::steps()),
    },
    Definition {
        version: 18,
        name: "offers",
        steps: || Ok(super::m0018_offers::steps()),
    },
    Definition {
        version: 19,
        name: "layout_off_fields",
        steps: || Ok(super::m0019_layout_off_fields::steps()),
    },
    Definition {
        version: 20,
        name: "mention_read_at",
        steps: || Ok(super::m0020_mention_read_at::steps()),
    },
    Definition {
        version: 21,
        name: "kebab_words",
        steps: || Ok(super::m0021_kebab_words::steps()),
    },
    Definition {
        version: 22,
        name: "text_index",
        steps: || Ok(super::m0022_text_index::steps()),
    },
    Definition {
        version: 23,
        name: "search_indexes",
        steps: || Ok(super::m0023_search_indexes::steps()),
    },
    Definition {
        version: 24,
        name: "saved_searches",
        steps: || Ok(super::m0024_saved_searches::steps()),
    },
    Definition {
        version: bb_core::text_index::RULE_MIGRATION,
        name: "text_in_nfc",
        steps: || Ok(super::m0025_text_in_nfc::steps()),
    },
    Definition {
        version: 26,
        name: "name_key",
        steps: || Ok(super::m0026_name_key::steps()),
    },
    Definition {
        version: 27,
        name: "person_setting_version",
        steps: || Ok(super::m0027_person_setting_version::steps()),
    },
    Definition {
        version: 28,
        name: "person_setting_counter",
        steps: super::m0028_person_setting_counter::steps,
    },
    Definition {
        version: 29,
        name: "notification_by_time",
        steps: || Ok(super::m0029_notification_by_time::steps()),
    },
    Definition {
        version: 30,
        name: "saved_search_spacing",
        steps: || Ok(super::m0030_saved_search_spacing::steps()),
    },
    Definition {
        version: 31,
        name: "time_entries",
        steps: || Ok(super::m0031_time_entries::steps()),
    },
    Definition {
        version: 32,
        name: "working_hours",
        steps: super::m0032_working_hours::steps,
    },
    Definition {
        version: 33,
        name: "issue_estimates",
        steps: || Ok(super::m0033_issue_estimates::steps()),
    },
    Definition {
        version: 34,
        name: "layout_time_fields",
        steps: super::m0034_layout_time_fields::steps,
    },
    Definition {
        version: 35,
        name: "choice_numbers",
        steps: || Ok(super::m0035_choice_numbers::steps()),
    },
    Definition {
        version: 36,
        name: "issue_totals_index",
        steps: || Ok(super::m0036_issue_totals_index::steps()),
    },
    Definition {
        version: 37,
        name: "ai_agreement",
        steps: || Ok(super::m0037_ai_agreement::steps()),
    },
    Definition {
        version: 38,
        name: "ai_tools",
        steps: || Ok(super::m0038_ai_tools::steps()),
    },
    Definition {
        version: 39,
        name: "agents",
        steps: || Ok(super::m0039_agents::steps()),
    },
    Definition {
        version: 40,
        name: "runs",
        steps: || Ok(super::m0040_runs::steps()),
    },
    Definition {
        version: 41,
        name: "assistant_references",
        steps: || Ok(super::m0041_assistant_references::steps()),
    },
    Definition {
        version: 42,
        name: "agent_work",
        steps: || Ok(super::m0042_agent_work::steps()),
    },
    Definition {
        version: 43,
        name: "oauth",
        steps: || Ok(super::m0043_oauth::steps()),
    },
    Definition {
        version: 44,
        name: "shown_moments",
        steps: || Ok(super::m0044_shown_moments::steps()),
    },
    Definition {
        version: 45,
        name: "agent_pause",
        steps: || Ok(super::m0045_agent_pause::steps()),
    },
    Definition {
        version: 46,
        name: "answer_time",
        steps: || Ok(super::m0046_answer_time::steps()),
    },
    Definition {
        version: 47,
        name: "work_project",
        steps: || Ok(super::m0047_work_project::steps()),
    },
    Definition {
        version: 48,
        name: "rules_hand_over",
        steps: || Ok(super::m0048_rules_hand_over::steps()),
    },
    Definition {
        version: 49,
        name: "dates_and_date_effect",
        steps: || Ok(super::m0049_dates_and_date_effect::steps()),
    },
    Definition {
        version: 50,
        name: "work_times",
        steps: || Ok(super::m0050_work_times::steps()),
    },
    Definition {
        version: 51,
        name: "own_themes",
        steps: || Ok(super::m0051_own_themes::steps()),
    },
    Definition {
        version: 52,
        name: "time_factor_and_accepted_warnings",
        steps: || Ok(super::m0052_time_factor_and_accepted_warnings::steps()),
    },
    Definition {
        version: 53,
        name: "issue_dates_index",
        steps: || Ok(super::m0053_issue_dates_index::steps()),
    },
    Definition {
        version: 54,
        name: "backup_runs",
        steps: || Ok(super::m0054_backup_runs::steps()),
    },
    Definition {
        version: 55,
        name: "data_exports",
        steps: || Ok(super::m0055_data_exports::steps()),
    },
    Definition {
        version: 56,
        name: "keeping",
        steps: || Ok(super::m0056_keeping::steps()),
    },
    Definition {
        version: super::m0057_person_standing::VERSION,
        name: "person_standing",
        steps: || Ok(super::m0057_person_standing::steps()),
    },
    Definition {
        version: 58,
        name: "e_mail_changes",
        steps: || Ok(super::m0058_e_mail_changes::steps()),
    },
    Definition {
        version: 59,
        name: "ai_switches_kept",
        steps: || Ok(super::m0059_ai_switches_kept::steps()),
    },
    Definition {
        version: 60,
        name: "standing_by_optional",
        steps: || Ok(super::m0060_standing_by_optional::steps()),
    },
    Definition {
        version: 61,
        name: "agent_requests",
        steps: || Ok(super::m0061_agent_requests::steps()),
    },
    Definition {
        version: 62,
        name: "agent_switches",
        steps: || Ok(super::m0062_agent_switches::steps()),
    },
    Definition {
        version: 63,
        name: "timer_by_issue",
        steps: || Ok(super::m0063_timer_by_issue::steps()),
    },
    Definition {
        version: 64,
        name: "walk_index",
        steps: || Ok(super::m0064_walk_index::steps()),
    },
    Definition {
        version: 65,
        name: "releases",
        steps: super::m0065_releases::steps,
    },
    Definition {
        version: 66,
        name: "board_calendar_card",
        steps: || Ok(super::m0066_board_calendar_card::steps()),
    },
    Definition {
        version: 67,
        name: "dashboards",
        steps: super::m0067_dashboards::steps,
    },
];
