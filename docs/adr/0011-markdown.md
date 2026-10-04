# ADR-0011: One Markdown renderer for server and browser

Status: **Accepted** · 2026-10-04

## Context
Issue descriptions, comments and documents are Markdown written by people, so they're untrusted. The preview must match what others will see.

## Decision
`bb-markdown` uses **pulldown-cmark** and sanitizes with **ammonia** (allow list), turns issue IDs into chips (resolved later with permission checks), and blocks remote images unless an admin allows the host. The same crate runs on the server and, via WebAssembly, in the browser for previews.

## Consequences
- One sanitizer configuration (SSOT); no client-side HTML rendering library.
- The document editor (Tiptap) stores Markdown; its schema is limited to what `bb-markdown` renders.

## Alternatives considered
- **marked/markdown-it + DOMPurify in the browser:** a second renderer that could disagree with the server. Rejected.

## Enforced by
ARCH-PURE, ARCH-WASM, SEC-PATTERNS (see `tools/checks/catalog.toml`).
