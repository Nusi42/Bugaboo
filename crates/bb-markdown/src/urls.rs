//! Which addresses may be in the HTML (SEC-13): the only place that decides it.
//!
//! The decisions are made on the text of the address, without the `url` crate: that crate brings
//! the tables of international domain names, which would make the WebAssembly module far bigger
//! than its budget (PERF-04). The rules are strict instead of clever: what is not clearly fine is
//! not allowed.

/// The only kinds of link that may be followed: the web, and mail.
const LINK_SCHEMES: [&str; 3] = ["http", "https", "mailto"];
/// The only scheme of an image from outside.
const IMAGE_SCHEME: &str = "https";
/// What comes after the scheme of an address with a host.
const AUTHORITY_MARK: &str = "//";
/// The start of a path of our own origin; a second slash would be another host.
const OWN_ORIGIN_PREFIX: &str = "/";
/// Where the host of an address ends.
const HOST_END: [char; 3] = ['/', '?', '#'];
/// Characters besides letters and digits that a host (with its port) may have. Names with other
/// letters are written the way the browser sends them (punycode).
const HOST_MARKS: [char; 3] = ['.', '-', ':'];

/// An address without spaces, control characters or backslashes. Browsers strip tabs and line
/// breaks inside addresses and read `\` as `/`, which would hide a scheme or turn `/\host` into
/// another host.
fn plain(url: &str) -> bool {
    !url.chars()
        .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
}

/// The scheme of an address, if it has one: letters, digits, `+`, `-` and `.` before the first `:`,
/// starting with a letter, and that `:` before any `/`, `?` or `#`.
fn scheme_of(url: &str) -> Option<&str> {
    let end = url.find(':')?;
    let scheme = &url[..end];
    let valid = scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c));
    valid.then_some(scheme)
}

/// Whether a link address may stay: an address of ours (no scheme, and not `//host`), or one of the
/// allowed schemes.
pub(crate) fn link_allowed(url: &str) -> bool {
    if !plain(url) || url.starts_with(AUTHORITY_MARK) {
        return false;
    }
    scheme_of(url).is_none_or(|scheme| {
        LINK_SCHEMES
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(scheme))
    })
}

/// Whether an image may be shown: a path of our own origin, or an `https` address on one of the
/// hosts the organization allows (the setting `content.image-hosts`; a host with a port must be
/// written with it).
pub(crate) fn image_allowed(url: &str, hosts: &[String]) -> bool {
    if !plain(url) {
        return false;
    }
    if let Some(path) = url.strip_prefix(OWN_ORIGIN_PREFIX) {
        return !path.starts_with('/');
    }
    let Some(scheme) = scheme_of(url) else {
        return false;
    };
    let Some(rest) = url[scheme.len() + 1..].strip_prefix(AUTHORITY_MARK) else {
        return false;
    };
    let host = rest.split(HOST_END).next().unwrap_or_default();
    scheme.eq_ignore_ascii_case(IMAGE_SCHEME)
        && !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || HOST_MARKS.contains(&c))
        && hosts
            .iter()
            .any(|allowed| allowed.trim().eq_ignore_ascii_case(host))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hosts(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn own_origin_and_allowed_hosts_may_show_images() {
        let allowed = hosts(&["images.example.com", "cdn.example.com:8443"]);
        assert!(image_allowed("/files/a.png", &[]));
        assert!(image_allowed("https://images.example.com/a.png", &allowed));
        assert!(image_allowed("HTTPS://IMAGES.example.com?x=1", &allowed));
        assert!(image_allowed(
            "https://cdn.example.com:8443/a.png",
            &allowed
        ));
    }

    #[test]
    fn other_images_may_not() {
        let allowed = hosts(&["images.example.com"]);
        for url in [
            "//images.example.com/a.png",
            "/\\evil.example/a.png",
            "http://images.example.com/a.png",
            "https://evil.example/a.png",
            "https://images.example.com.evil.example/a.png",
            "https://user@images.example.com/a.png",
            "https://images.example.com:444/a.png",
            "https://images.example.com%2eevil.example/a.png",
            "https:images.example.com/a.png",
            "https:///a.png",
            "data:image/png;base64,AAAA",
            "javascript:alert(1)",
            "a.png",
            "https://images.example.com/a b.png",
            "",
        ] {
            assert!(!image_allowed(url, &allowed), "{url}");
        }
    }

    #[test]
    fn links_go_to_our_own_pages_or_to_the_allowed_schemes() {
        for url in [
            "/issues/WEB-1",
            "https://example.com",
            "HTTP://example.com",
            "mailto:ana@example.com",
            "#top",
            "?page=2",
            "notes/today",
        ] {
            assert!(link_allowed(url), "{url}");
        }
    }

    #[test]
    fn links_never_run_code_or_hide_another_host() {
        for url in [
            "//evil.example",
            "/\\evil.example",
            "javascript:alert(1)",
            "JaVaScRiPt:alert(1)",
            "java\tscript:alert(1)",
            " javascript:alert(1)",
            "data:text/html,x",
            "vbscript:x",
            "file:///etc/passwd",
            "ftp://example.com",
        ] {
            assert!(!link_allowed(url), "{url:?}");
        }
    }
}
