//! The one list of what may be bold (`bold { … }` in `design/theme.kdl`, UX §7.3, STY-BOLD).

use anyhow::bail;

use super::kdl_reader::Node;

/// What a bold says. A new role needs UX §7.3 first; the words are written here once, and the
/// theme file and the check read them from here.
pub(crate) const BOLD_ROLES: [&str; 5] = ["issue-title", "title", "section", "priority", "own"];

/// One selector of a rule of the app that may set the bold weight, with the role that allows it.
pub(crate) struct BoldSelector {
    pub(crate) selector: String,
    pub(crate) role: String,
    pub(crate) line: usize,
}

pub(crate) struct BoldSpec {
    pub(crate) selectors: Vec<BoldSelector>,
}

impl BoldSpec {
    pub(crate) fn parse(node: Node<'_>) -> anyhow::Result<Self> {
        if node.required_prop("reason")?.trim().is_empty() {
            bail!("line {}: the `bold` list needs a reason", node.line());
        }
        let mut selectors = Vec::new();
        for child in node.children() {
            if child.name() != "selector" {
                bail!("line {}: `bold` holds `selector` lines only", child.line());
            }
            let role = child.required_prop("role")?;
            if !BOLD_ROLES.contains(&role.as_str()) {
                bail!(
                    "line {}: `{role}` is no role of a bold; the roles are {}",
                    child.line(),
                    BOLD_ROLES.join(", ")
                );
            }
            selectors.push(BoldSelector {
                selector: child.only_arg()?,
                role,
                line: child.line(),
            });
        }
        Ok(Self { selectors })
    }
}
