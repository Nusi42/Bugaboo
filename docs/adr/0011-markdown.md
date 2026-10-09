# ADR-0011: One Markdown renderer for server and browser

Status: **Accepted** · 2026-10-04 · changed (M3 WP-02: the writer replaces ammonia, see "Change")

## Context

Issue descriptions, comments and documents are Markdown written by people, so they're untrusted. The preview must match what others will see.

## Decision

`bb-markdown` uses **pulldown-cmark** to parse and **writes the HTML itself** from the parser's events, with one allow list: only the tags and attributes of its writer can be in the output, raw HTML in the text is turned into text, links get `rel="noopener noreferrer"` and open outside, only `http`, `https`, `mailto` and our own pages are links, and images come from our own origin or from a host an admin allows. Issue IDs and mentions come back as tokens (resolved later with permission checks), shown as chips. The same crate runs on the server and, via WebAssembly, in the browser for previews.

## Consequences

- One allow list (SSOT), in the writer; no client-side HTML rendering library.
- Nothing is parsed back and cleaned, so there is no gap between what the writer meant and what a sanitizer's HTML parser reads (mutation XSS has no place to start).
- The WebAssembly module stays small: the budget of the module is in `tools/checks/perf-bundle.toml` (architecture §8, PERF-04), and the search language shares it.
- The browser shows the HTML with one Trusted Types policy, `bugaboo-markdown`, in the one component `MarkdownView` (SEC-RICHTEXT).
- The document editor (Tiptap) stores Markdown; its schema is limited to what `bb-markdown` renders.

## Change (M3 WP-02)

The first plan was to sanitize with **ammonia** after pulldown-cmark's own HTML. Measured on the real module: ammonia brings an HTML parser (html5ever), the URL library with its tables for international domain names, and their data, about **250 KB more with Brotli**, so the module would be about four times the size of the parser alone, far over its budget, which the search language (M5) has to share. Writing the HTML from the events needs none of that, is easier to read as one allow list, and keeps the security property: the output holds only what the writer can write. The corpus test (SEC-13) and the property tests check the output for tags, attributes and addresses that the allow list does not have.

## Alternatives considered

- **marked/markdown-it + DOMPurify in the browser:** a second renderer that could disagree with the server. Rejected.
- **pulldown-cmark + ammonia (the first plan):** a clean-up after the HTML is made; too big for WebAssembly (see "Change"). Rejected for the browser module, and one renderer must serve both.

## Enforced by

ARCH-PURE, ARCH-WASM, SEC-PATTERNS, SEC-RICHTEXT, PERF-BUNDLE (the budget of the module) (see `tools/checks/catalog.toml`).
