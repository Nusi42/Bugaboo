# WP-02 — Safe Markdown on the server and in the browser

**Wave 1 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** SEC-13; the rendering part of ISS-01, ISS-05, CF-01 (long text); UX design §4.4, §7; ADR-0011, ADR-0005; follow-ups M1-F10, M2-F03.

## Goal

Every piece of text a person writes (descriptions, comments, long-text fields) is turned into safe HTML by one crate, on the server and in the browser's preview, so a script in a comment shows as text and never runs.

## Owns

`crates/bb-markdown/**`, `crates/bb-wasm/**`, the WebAssembly build and its budget in `xtask` (`arch_wasm.rs`, PERF-BUNDLE's WebAssembly part), `web/packages/ui/src/markdown/**` (new: the view and the editor with preview), the `markdown` parts in `design/theme.kdl`, the new checks below, `docs/third-party/README.md` rows for `ammonia`, `wasm-bindgen` and product use of `pulldown-cmark` (versions pinned in architecture §9), `userDocs/writing/*` (new: what Markdown you can use).

## Tasks

1. **`bb-markdown`:** `render(text, &Options) -> Rendered` with pulldown-cmark and one ammonia allowlist (ADR-0011): no raw HTML, no `javascript:` or `data:` links, images only from our own origin and the hosts an organization allows (`Options`), links get `rel="noopener noreferrer"` and open outside. Issue IDs (`WEB-142`, `WEB-K7Q2X`) and mentions (`@…`) come back as *tokens* with their place in the text; the server resolves them with permission checks (WP-07) and the browser shows them as chips. No I/O and no dependency on `bb-core` (dependency rule 1). Limits (text length, nesting depth) as named constants.
2. **`bb-wasm`:** `wasm-bindgen` exports of `render` for the browser; built with `wasm-opt`; loaded only when a preview or an editor opens. Budget 150 KB Brotli (architecture §8) as a named constant read by the check.
3. **Browser:** `MarkdownView` shows rendered HTML from the server (the one place that sets HTML from a string, through a Trusted Types policy named in the CSP); `MarkdownEditor` is a text box with a small toolbar (bold, italic, list, link, code, quote) and a *Preview* tab that calls `bb-wasm`. No other renderer in the browser.
4. **Allowed image hosts:** an organization setting (`content.image-hosts`, a list, empty by default), declared in the settings registry and shown in *Organization settings › Security*.

## Acceptance tests (write first)

- `sec_13_the_sanitizer_corpus_renders_every_attack_as_text`: a corpus file (`crates/bb-markdown/tests/corpus/*.md` with the expected HTML) with `<script>`, `<img src=x onerror=…>`, `javascript:` and `data:` links, SVG, entity tricks, nested Markdown/HTML mixes, remote images with and without an allowed host.
- `sec_13_server_and_browser_render_the_same_html`: the WebAssembly build and the native build give equal output for the whole corpus (run under `wasm-bindgen-test` or Node).
- `iss_01_issue_ids_and_mentions_come_back_as_tokens_with_their_places` (property test: tokens never overlap and point into the text).
- Playwright: write in the editor, see the preview in four themes, axe; a pasted `<script>` shows as text.

## Checks

- **SEC-RICHTEXT (new):** the corpus test above runs in CI, and every route that stores user text renders it only through `bb-markdown` (an xtask rule: no other crate depends on `pulldown-cmark` or `ammonia`; no `innerHTML` or `dangerouslySetInnerHTML` outside `MarkdownView`). The four parts: the rule in coding-standards §4, the catalogue entry, the implementation, bad and good examples.
- **ARCH-PURE** (planned → active): `bb-core`, `bb-searchlang`, `bb-markdown` use no file, network, clock or random I/O.
- **SEC-PATTERNS** (planned → active), as the catalogue describes it.
- **ARCH-WASM:** adds `bb-wasm` and the size budget (PERF-04).

## Depends on

Nothing in M3.
