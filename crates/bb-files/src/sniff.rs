//! The real type of a file, read from its first bytes (ATT-03, ADR-0032 §2).
//!
//! The name and what the browser claims are never looked at here. `infer` knows the magic numbers
//! of the common formats; a file it does not know (plain text, a script, an SVG or an HTML page)
//! is just data, and every download is sent as an attachment anyway.

use bb_app::files::Sniffed;
use bb_core::files::MediaType;

/// How many bytes of a file are enough to tell its type: the longest signature `infer` reads is
/// far shorter, but some container formats put theirs a few kilobytes in.
pub(crate) const HEAD_BYTES: usize = 8192;

/// What the first bytes of a file say it is.
#[must_use]
pub fn sniff(head: &[u8]) -> Sniffed {
    let known = infer::get(&head[..head.len().min(HEAD_BYTES)]);
    Sniffed {
        media_type: known
            .and_then(|known| MediaType::new(known.mime_type()))
            .unwrap_or_else(MediaType::unknown),
        extension: known.map(|known| known.extension().to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use bb_core::files::UNKNOWN_MEDIA_TYPE;

    use super::*;

    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];

    #[test]
    fn att_03_the_type_comes_from_the_content_not_the_name() {
        let png = sniff(PNG);
        assert_eq!(png.media_type.as_str(), "image/png");
        assert_eq!(png.extension.as_deref(), Some("png"));
        let program = sniff(b"MZ\x90\x00\x03\x00\x00\x00\x04\x00\x00\x00\xff\xff\x00\x00");
        assert_eq!(program.extension.as_deref(), Some("exe"));
        let pdf = sniff(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n");
        assert_eq!(pdf.media_type.as_str(), "application/pdf");
    }

    #[test]
    fn att_02_svg_is_just_data_and_html_and_scripts_are_what_they_are() {
        for text in [
            &b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>"[..],
            b"plain words",
            b"",
        ] {
            let found = sniff(text);
            assert_eq!(found.media_type.as_str(), UNKNOWN_MEDIA_TYPE);
            assert_eq!(found.extension, None);
        }
        // HTML and shell scripts are known by their start. They are stored as what they are, and
        // no download ever sends that type (`bb_app::files::download`): they are saved, never run
        // or shown by the browser.
        let html = sniff(b"<!doctype html><script>alert(1)</script>");
        assert_eq!(html.media_type.as_str(), "text/html");
        let script = sniff(b"#!/bin/sh\nrm -rf /");
        assert_eq!(script.extension.as_deref(), Some("sh"));
    }
}
