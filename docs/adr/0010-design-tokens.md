# ADR-0010: Styling through design tokens only

Status: **Accepted** · 2026-10-04

## Context

Four built-in themes, a theme editor, custom field looks and strict CSP, all without letting customization inject code.

## Decision

- `design/tokens.json` (W3C Design Tokens format) is the **only source** of colors, fonts, sizes, radii and shadows, for all four themes.
- `cargo xtask gen tokens` produces: CSS variables per theme, TypeScript token names, and the Rust validation used by the theme editor (allowed values, contrast checks).
- Components use **CSS Modules** with `var(--token)` only. The hooks reject raw colors, pixel sizes outside tokens and inline styles.
- Custom themes and field looks store **token values or token names**, never CSS.

## Consequences

- The design pages in `docs/design/` use the generated CSS once the generator exists (today `tokens.css` is hand-written and becomes generated output).

## Alternatives considered

- **Tailwind CSS:** harder to enforce "tokens only"; class soup in agent-written code. Rejected.
- **CSS-in-JS at runtime:** costs performance and needs CSP exceptions. Rejected.

## Enforced by

STY-TOKENS, LINT-CSS, STY-CONTRAST (see `tools/checks/catalog.toml`).
