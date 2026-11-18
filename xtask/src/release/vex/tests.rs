use super::*;
use crate::checks::dep_register::parse_register;
use crate::release::{formats, repository_root, this_repository};

const DENY: &str = r#"
[advisories]
ignore = [
  { id = "RUSTSEC-2023-0001", reason = "short reason" },
  "RUSTSEC-2023-0002",
]
"#;

const NPM: &str = r#"
accepted = [
  { id = "GHSA-aaaa-bbbb-cccc", reason = "an npm reason" },
]
"#;

const REGISTER: &str = "\
## Accepted risks

| Advisory | Package | Severity | Why accepted | Re-check |
|----------|---------|----------|--------------|----------|
| [RUSTSEC-2023-0001](https://rustsec.org/advisories/RUSTSEC-2023-0001) | foo 1.0 | low | Only public keys are used. | later |
";

#[test]
fn entries_are_read_from_both_shapes() {
    let ids: Vec<_> = entries(DENY, &["advisories", "ignore"])
        .unwrap()
        .into_iter()
        .map(|entry| entry.id)
        .collect();
    assert_eq!(ids, ["RUSTSEC-2023-0001", "RUSTSEC-2023-0002"]);
    assert_eq!(
        entries(NPM, &["accepted"]).unwrap()[0].reason,
        "an npm reason"
    );
}

#[test]
fn a_statement_says_not_affected_with_the_reason_written_in_the_register() {
    let list = &this_repository().list;
    let mut all = entries(DENY, &["advisories", "ignore"]).unwrap();
    all.extend(entries(NPM, &["accepted"]).unwrap());
    let document = vex(list, &all, &parse_register(REGISTER));
    let found = document["vulnerabilities"].as_array().unwrap();
    assert_eq!(found.len(), 3);
    assert_eq!(found[0]["analysis"]["state"], "not_affected");
    assert_eq!(found[0]["analysis"]["detail"], "Only public keys are used.");
    assert_eq!(found[0]["description"], "foo 1.0");
    assert_eq!(
        found[0]["source"]["url"],
        "https://rustsec.org/advisories/RUSTSEC-2023-0001"
    );
    assert_eq!(
        found[2]["analysis"]["detail"], "an npm reason",
        "own reason as the fallback"
    );
    assert_eq!(
        formats::cyclonedx_errors(&document).unwrap(),
        Vec::<String>::new()
    );
}

#[test]
fn comp_03_every_accepted_advisory_has_a_vex_statement_with_its_reason() {
    let root = repository_root();
    let accepted = accepted(&root).unwrap();
    assert!(
        accepted.len() >= 3,
        "deny.toml and the npm list accept three"
    );
    let register = crate::checks::dep_register::read_register(&root).unwrap();
    let files = this_repository();
    let statements = files.vex["vulnerabilities"].as_array().unwrap();
    assert_eq!(statements.len(), accepted.len());
    for entry in &accepted {
        let statement = statements
            .iter()
            .find(|statement| statement["id"] == entry.id.as_str())
            .unwrap_or_else(|| panic!("{} has no VEX statement", entry.id));
        assert_eq!(statement["analysis"]["state"], "not_affected");
        let detail = statement["analysis"]["detail"].as_str().unwrap();
        assert!(!detail.is_empty(), "{} has no reason", entry.id);
        // The reason is the register's, written once, when the register describes the risk.
        if let Some(row) = register.risks.get(&entry.id) {
            assert_eq!(detail, row.purpose);
        }
        assert!(
            register.risks.contains_key(&entry.id),
            "{} has no row in the register's Accepted risks",
            entry.id
        );
    }
    assert!(formats::cyclonedx_errors(&files.vex).unwrap().is_empty());
}
