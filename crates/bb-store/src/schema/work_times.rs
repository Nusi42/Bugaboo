//! The tables of work times (PLAN-11, PLAN-15, ADR-0030, decision 2): the usual work days of a
//! person, their share in each project and their away times. There is no column for a reason: an
//! away time does not say why (GDPR Art. 9).

use super::{Iden, StoredTable};

/// The usual work days of a person: one row for each weekday of a person who changed them. The
/// person has no reference: a person's work times are deleted by the one function that deletes them
/// when the person leaves.
#[derive(Clone, Copy, Iden)]
pub(crate) enum UsualWorkDayRow {
    #[iden = "usual_work_day"]
    Table,
    Person,
    Weekday,
    Works,
    Minutes,
}

/// The share of a person in a project: one row for each share that was set.
#[derive(Clone, Copy, Iden)]
pub(crate) enum ProjectShareRow {
    #[iden = "project_share"]
    Table,
    Person,
    Project,
    Percent,
}

/// The away times of the people.
#[derive(Clone, Copy, Iden)]
pub(crate) enum AwayTimeRow {
    #[iden = "away_time"]
    Table,
    Id,
    Person,
    First,
    Last,
    FirstAfternoonOnly,
    LastMorningOnly,
    Note,
    EnteredBy,
    Version,
}

stored_tables! {
    UsualWorkDayRow => bb_core::work_times::record::UsualWorkDay {
        Person => person,
        Weekday => weekday,
        Works => works,
        Minutes => minutes,
    },
    ProjectShareRow => bb_core::work_times::record::ProjectShare {
        Person => person,
        Project => project,
        Percent => percent,
    },
    AwayTimeRow => bb_core::work_times::record::AwayTime {
        Id => id,
        Person => person,
        First => first,
        Last => last,
        FirstAfternoonOnly => first_afternoon_only,
        LastMorningOnly => last_morning_only,
        Note => note,
        EnteredBy => entered_by,
        Version => version,
    },
}

#[cfg(test)]
mod tests {
    use super::tables;

    // An away time says nothing of why: a reason could be health data (GDPR Art. 9, ADR-0030,
    // decision 2). The review looks for one too; this holds the tables to it.
    #[test]
    fn plan_11_no_table_of_work_times_has_a_reason() {
        let tables = tables();
        let names: Vec<&str> = tables.iter().map(|table| table.name.as_str()).collect();
        assert_eq!(names, ["usual_work_day", "project_share", "away_time"]);
        for table in &tables {
            for (column, field) in &table.columns {
                for text in [column.as_str(), *field] {
                    assert!(
                        !text.to_lowercase().contains("reason")
                            && !text.to_lowercase().contains("kind")
                            && !text.to_lowercase().contains("type"),
                        "{} has `{text}`",
                        table.name
                    );
                }
            }
        }
    }
}
