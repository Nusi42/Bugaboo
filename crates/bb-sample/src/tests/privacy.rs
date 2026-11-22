//! The acceptance test of the people who left, the export and the backups in the sample (M9 part 8,
//! PRIV-03, PRIV-04, OPS-07, TEST-SAMPLE-DATA): one person switched off, one removed whose comments
//! stay as *Former user* and whose work days are kept under the legal-hold name, the file of Lea
//! Fischer's data ready, and two backups. They make the real thing on SQLite and PostgreSQL and read
//! the canonical dump.

use bb_dev_words::sample::Layout;
use bb_testing::{TestResult, each_database};

use super::work::{ISSUES_ASKED as ISSUES, dump_of};
use crate::Target;

#[tokio::test]
async fn priv_04_the_sample_has_a_switched_off_and_a_removed_person() -> TestResult {
    Box::pin(each_database(
        "priv_04_the_sample_has_a_switched_off_and_a_removed_person",
        async |db| {
            let folder = tempfile::tempdir()?;
            let dump = Box::pin(dump_of(
                &Target::Address(db.url().to_owned()),
                folder.path(),
            ))
            .await?;
            // TEST-SAMPLE-DATA: the plan judges the part of the dump on the people who left.
            let plan = Layout::bundled()?.issues;
            let problems = plan.problems(ISSUES, "sample", &dump);
            assert!(problems.is_empty(), "{problems:?}");
            let privacy = &dump["issues"]["work"]["privacy"];
            assert_eq!(privacy["switched_off"], 1);
            assert_eq!(privacy["removed"]["count"], 1);
            assert_eq!(privacy["removed"]["name_held_until_a_day"], true);
            assert_eq!(privacy["export"]["person"], "Lea Fischer");
            assert_eq!(privacy["export"]["state"], "ready");
            assert_eq!(privacy["backups"], 2);
            Ok(())
        },
    ))
    .await
}
