//! The addresses of the pages of the browser app that the server links to (invitations, e-mails),
//! written once. The app's own list of addresses (`web/app/src/shared/paths.ts`) takes these from
//! the file that `cargo xtask gen api` writes (`app-paths.generated.ts`), so the link in an e-mail
//! and the page it opens cannot differ (ODR).

/// The name of the changing part of [`ISSUE`].
pub const ISSUE_PARAM: &str = "issue";

/// The page of an issue; the readable ID of the issue takes the place of `$issue`.
pub const ISSUE: &str = "/issues/$issue";

/// The page that takes an invitation link (the secret follows the `#`, so it never reaches a log).
pub const INVITATION: &str = "/invitation";

/// The page to sign in.
pub const SIGN_IN: &str = "/sign-in";

/// The address of an issue page from the readable ID of the issue.
#[must_use]
pub fn issue(key: &str) -> String {
    ISSUE.replace(&format!("${ISSUE_PARAM}"), key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_address_of_an_issue_has_its_readable_id_in_it() {
        assert_eq!(issue("WEB-142"), "/issues/WEB-142");
    }
}
