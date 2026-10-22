//! ARCH-WEB-KEYS: the browser writes no query key and no server size of its own (architecture §4,
//! "One home for what several screens share").
//!
//! * The keys of the query cache are written in `web/app/src/shared/query-keys.ts` and nowhere
//!   else. A key written in a feature (`queryKey: ["get", "/api/v1/…"]`) is a second definition
//!   that a change of the route or of the cache can leave behind, so a feature asks a `refresh…`
//!   function or a key of that file. The keys the generated client makes for itself are taken from
//!   its options (`xOptions().queryKey`), which is not a literal.
//! * The end-to-end tests take the sizes the server decides (the part of an upload, a megabyte)
//!   from the generated files of `@bugaboo/api-client` and never write `1024 * 1024` again.

use super::web_source::code_of;
use super::{Check, Context, Finding};
use crate::source::{Kind, Source};

/// The one file that writes keys.
const KEYS_HOME: &str = "web/app/src/shared/query-keys.ts";
const APP_SOURCE: &str = "web/app/src/";
const E2E: &str = "web/app/e2e/";

pub(crate) struct ArchWebKeys;

impl Check for ArchWebKeys {
    fn id(&self) -> &'static str {
        "ARCH-WEB-KEYS"
    }

    fn run(&self, context: &Context<'_>) -> anyhow::Result<Vec<Finding>> {
        Ok(evaluate(&context.sources(self.id(), Kind::Web)?))
    }
}

fn evaluate(sources: &[Source]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (path, text) in sources {
        let is_test = path.contains(".test.");
        let in_app = path.starts_with(APP_SOURCE) && path != KEYS_HOME && !is_test;
        let in_e2e = path.starts_with(E2E);
        if !in_app && !in_e2e {
            continue;
        }
        for (index, line) in text.lines().enumerate() {
            let code: String = code_of(line)
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            let at = format!("{path}:{}", index + 1);
            if in_app && writes_a_key(&code) {
                findings.push(Finding::at(
                    at,
                    "A query key is written here. Keys live in shared/query-keys.ts only: add a key or a `refresh…` function there and use it.",
                ));
            } else if code.contains("1024*1024") {
                findings.push(Finding::at(
                    at,
                    "A size the server decides is written again. Take `partBytes` or `megabyte` from `@bugaboo/api-client` (generated from bb-core).",
                ));
            }
        }
    }
    findings
}

/// Whether a line (without spaces) writes a key as an array: `queryKey:[…` or `["get",…`.
fn writes_a_key(code: &str) -> bool {
    code.contains("queryKey:[")
        || ["get", "post", "put", "patch", "delete"]
            .iter()
            .any(|method| code.contains(&format!("[\"{method}\",\"/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(path: &str, text: &str) -> usize {
        evaluate(&[(path.to_owned(), text.to_owned())]).len()
    }

    #[test]
    fn good_example_keys_in_their_home_and_options_of_the_client() {
        assert_eq!(
            found(KEYS_HOME, "export const k = { a: { queryKey: [\"a\"] } };"),
            0
        );
        assert_eq!(
            found(
                "web/app/src/features/x/x.ts",
                "queryClient.invalidateQueries({ queryKey: statusOptions().queryKey });\n// queryKey: [\"get\", \"/x\"]"
            ),
            0
        );
        assert_eq!(
            found(
                "web/app/e2e/support/files.ts",
                "import { partBytes } from \"@bugaboo/api-client\";"
            ),
            0
        );
    }

    #[test]
    fn bad_example_a_key_written_in_a_feature_or_a_size_in_a_test() {
        assert_eq!(
            found(
                "web/app/src/features/files/files.ts",
                "  queryKey: [\"get\", \"/api/v1/issues/{issue}/files\"],"
            ),
            1
        );
        assert_eq!(
            found(
                "web/app/src/features/a.ts",
                "q({ queryKey: [\"people\", id] })"
            ),
            1
        );
        assert_eq!(
            found(
                "web/app/e2e/support/files.ts",
                "const MEGABYTE = 1024 * 1024;"
            ),
            1
        );
    }
}
