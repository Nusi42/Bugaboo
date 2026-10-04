// Hand-written HTML pages (docs/02-ux-design.html, docs/mockups/*.html) are artifact pages: the
// host wraps them, so they have no doctype, html, head or body. html-validate accepts that.
export default {
  extends: ["html-validate:recommended", "html-validate:a11y"],
  rules: {
    // The design pages draw illustrative mini-screens with per-element values, so inline styles
    // are part of that format. The real app is covered by LINT-CSS, which bans them (ADR-0010).
    "no-inline-style": "off",
  },
};
