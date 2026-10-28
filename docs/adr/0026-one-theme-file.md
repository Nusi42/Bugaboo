# ADR-0026: One theme file for everything about how the app looks

Status: **Accepted** · 2026-10-08 · replaces the styling part of [ADR-0010](0010-design-tokens.md)

## Context

Requirement in M2C: **all UI theming lives in one theming file**, the way the sister project keeps its complete look in one theme file. ADR-0010 made `design/tokens.json` the source of colors, fonts, sizes, radii and shadows, but let every component keep its own CSS Module. The audit of M2C (section D) found what that costs:

- 55 CSS modules with 3,261 lines; values that have no token (`font-weight: 700` 57 times, line heights 38 times, about 50 `1px solid` borders, a z-index chosen per file, opacities, keyframes);
- the same rule copied into many modules (the focus ring 22 times, the muted text, `.stack`, `.row`), the skip link twice, and `forced-colors` handling in ten components;
- the theme list written in `tokens.json` and again in the settings registry in Rust, held together by a test (M2C-F64);
- theme names and the `data-look` attribute written by hand in TypeScript and in the browser tests.

The sister project compiles its theme to CSS **in the browser** and injects it as a style element. Bugaboo cannot do that: the content security policy is `style-src 'self'` (SEC-11), so nothing may put a style on the page at run time.

## Decision

1. **One file, `design/theme.kdl`** (KDL, like the sister project; decision M2C-Q1). It holds everything: the themes, the choices that change token values, every token with its variants per mode, the contrast rules, the mixins, the keyframes and the rules of every component. Comments are allowed, and each token is one line. It replaces `design/tokens.json`. Only `cargo xtask gen theme` reads it (with the `kdl` crate, pinned like the sister project's); the server and the browser never parse KDL.
2. **Values are tokens, with their variants per mode on the line of the token.** A token has a kind (`color`, `font`, `size`, `number`, `duration`, `easing`, `shadow`, `weight`, `line-height`, `border-width`, `layer`, `opacity`) and, besides its value:
   - `<theme>=` for each of the four themes (colors and shadows), with `kind="light"` or `"dark"` on the theme;
   - `<theme>-more=` for a device that asks for more contrast (`prefers-contrast: more`);
   - `forced=` while forced colors are on (`forced-colors: active`);
   - `<choice>=` while a choice of the person is on: `compact` spacing, `larger-text`, `less-motion` (which the device can switch on too).

   Every token of ADR-0010 keeps its value; the file adds font weight, line height, border width, layers (z-index), opacity, easing, the brightness of a pressed primary button, and forced colors as a mode. The focus ring is two tokens (`ring-width`, `ring-offset`) and one mixin (`ring`) that every rule `use`s.
3. **Rules are `part` rules that use tokens only.** `part "button[variant=primary]:hover" { background "$accent" }` styles every element that says `data-part="button"` and `data-variant="primary"`. A value is a word, `0`, a percentage, or `$token` (written `var(--token)` in the output); a length, color, duration, weight, line height, layer or opacity written out is a finding of LINT-CSS and STY-TOKENS. `base` rules style the page frame (`html`, `body`); `mixin`, `keyframes` and `forced { … }`, `more-contrast { … }`, `media "…" { … }` blocks complete the language.
4. **Components carry no style values and no class names.** They emit `data-part`, `data-state` and `data-variant`, through `partOf(part.button, { variants: […], states: […] })` of `@bugaboo/ui`. The names of the parts and the words a part may have (`partWords`) are generated into `tokens.ts` from the rules, so a misspelled part or word is a type error. React Aria's own `data-*` state attributes (`data-hovered`, `data-pressed`, `data-selected`, `data-focus-visible`, `data-entering`, …) are allowed and used: a rule names them as `button[hovered]`.
5. **The file is compiled at build time into one static style sheet**, `web/packages/ui/src/theme.css` (committed, with a copy for the design pages at `docs/design/theme.css`). The content security policy stays `style-src 'self'`; no style element or attribute is made at run time. This is where Bugaboo differs from the sister project. Neither file is edited by hand: DOC-GENERATED compares them with the output of `cargo xtask gen theme`.
6. **The list of themes is written once.** The four built-in themes (Daylight, Paper, Dusk, Midnight) and the two that start are defined in the file. `gen theme` writes them to TypeScript (`tokens.ts`: `themeIds`, `lightThemeIds`, `darkThemeIds`, the defaults) and to Rust (`crates/bb-core/src/settings/themes.rs`: the choices of the settings *Light theme* and *Dark theme* and their defaults, which `catalog.rs` names). The reader of the settings registry follows those names, so the browser's generated choices and defaults come from the same file. There is no hand-kept list in `catalog.rs` and no test that holds two lists together (closes M2C-F64).
7. **Custom themes, later, store token values only**: names from the generated token list, values checked by the kind of the token. They never add a rule, a selector or CSS text. How the values reach the page without breaking the policy is decided when the theme editor is built.
8. **Contrast is checked per theme and per mode** (STY-CONTRAST), the way the sister project's contrast check does: every text color on every background in each theme, and with `<theme>-more=` the 7:1 and 3:1 rules of ADR-0010. The rules for it are the `contrast` and `distinct` lines of the same file.
   Bold is the same kind of rule: the file has one `bold` list that names the selectors that may set the bold weight, each with its role from UX §7.3 (STY-BOLD), so a bold that UX does not name cannot be added by accident.
9. **The design pages are generated too.** `docs/design/components.css`, `docpage.css`, `ux-design.css` and `mockup.css` style the documents and mockups in `docs/`, not the app (the last two hold what the UX page and the mockup had in a `<style>` block, and the classes that replaced their `style=` attributes). They are written in the same file as `docs` blocks: `rule` lines with plain CSS selectors (the pages use class names and elements, which parts must not) that use tokens only, and the tokens of the pages themselves (`doc-…`), which go to `docs/design/theme.css` and not to the app's sheet. `gen theme` writes both sheets, LINT-CSS reads them like the parts, and DOC-GENERATED compares them (M2C-F93, done in C-06). The motion of the pages follows the motion tokens, so there is no reduced-motion rule with `!important`. THEME-ONLY reads the pages of `docs/` too: no `<style>` element and no `style` attribute in the markup, unless the page's own script builds its value (a position or a color it computes, which is a style of the run time, M2C-F105).
10. **Moving was done in two steps.** M2C C-01 built the engine and moved the primitives of `packages/ui`. M2C C-06 moved every app feature, the shell and the other components (M2C-F90): there is no CSS Module, no `cx` and no exemption any more, and THEME-ONLY passes with nothing exempt. What several features said the same way is one part (`stack`, `text`, `muted`, `row`, `list`, `item`, `facts`, `hint`, …, M2C-F65), and the focus ring, the skip link and the forced-colors outline of the marked item (`marked`) are written once.
11. **The names of the page frame are written once.** The attributes of `<html>` that the sheets and the code use (`data-look`, `data-theme`, `data-preview-look`) are the `frame` line of the file. `gen theme` writes them to the sheets, to `tokens.ts` (`pageAttributes`), which `theme.ts`, the theme sample and the end-to-end specs read, and to `docs/design/frame.js` (`bugabooFrame`), which the scripts of the design pages read (M2C-F91). The choices (`data-spacing`, `data-text`, `data-motion`) are read from `variants` the same way.

## Update (M3 WP-16, the new look)

- **Choices of the device.** A `variant` may have only `media=` (no attribute on the page): `touch` (`(pointer: coarse), (width < $bp-tablet)`) gives a token its finger value (`size-control` 36 becomes 44, `size-row` 44 becomes 64), `phone` gives text 16. A media query may name a `$token` with a plain value.
- **Container queries.** A rule may hold `container "(min-width: $bp-window-side)" { … }`, written as `@container`: a window follows its own width, not the screen's (UX §6.3).
- **One run-time style.** The size a person drags a window to is two variables of the theme file (`window-width`, `window-height`). The issue list adds four, for the same reason (a size the browser measures or the person drags, which no token can hold): `list-columns`, the widths of the columns that the head and every row share; `list-gutter`, the width of the scroll bar the rows have now, so the head keeps the same room and the columns stand over each other; and `list-width`, the width of the head as the browser measured it (as wide as its columns need), which is given to the virtualized rows because they have no width of their own, so head and rows scroll sideways as one; and `size-code-char`, one character of the code font as the browser draws it, measured from a probe in that font (the font of the computer may be any of its stack), so the ID column is as wide as the IDs really are. THEME-ONLY allows exactly `element.style.setProperty(variableOf("…"), value)`: `variableOf` only takes the name of a token, and nothing else may set a style from code.
- **The pictures use the parts.** `cargo xtask gen theme` also writes `docs/design/parts.css`, the rules of the parts without the tokens, so the pictures of the UX design are drawn with the same `data-part` markup as the app and cannot differ from it.
- **Several `distinct` rules.** The hovered and the selected row must differ from the tint of a status group as well as from the plain surfaces, so the file has one `distinct` rule for the plain surfaces and one for each tint.

## Consequences

- A change of look is a change in one file, and the generated sheet shows it in the diff. A rule exists once: the focus ring, the muted text, the field label.
- Writing a component needs no CSS file and no class name. A style that the person's choices (spacing, text size, motion, contrast) cannot reach cannot be written.
- `cargo xtask gen theme` is part of the build of the browser app and of DOC-GENERATED. The generated sheet is committed, so the server's embedded app needs no extra build step.
- Specificity is that of attribute selectors, so the order in the file is the order in the sheet: a rule for a state is written after the rules for the variants it must win over. Enforced by: review (the order is seen in the diff of the file; the screenshots of the showcase show a change of look).
- Rules for forced colors move to tokens (`forced=`) where one system color serves everywhere; a part keeps a `forced { … }` block where its own colors differ (a disabled button, a link).

## Alternatives considered

- **Keep JSON (`design/theme.json`):** no new dependency, but no comments, long, and the variants of a value would not sit on one line. KDL was chosen.
- **The sister project's way, compiling in the browser:** needs a style element or `adoptedStyleSheets` from a string, which `style-src 'self'` forbids. Rejected.
- **Keep CSS Modules next to the token file:** the situation of the audit. Rejected.
- **A hand-written style sheet with the same data attributes:** the values and the rules would again be in two places that no tool joins, and the theme list could not be generated from it.
- **Tailwind CSS and CSS-in-JS at run time:** rejected in ADR-0010 and still.

## Enforced by

THEME-ONLY (no `style=`, no `className`, no CSS file but the generated one), STY-TOKENS (no color written out), STY-PARTS (every part a rule names is a part some component has), LINT-CSS (the compiled rules use token variables only), STY-CONTRAST (every theme and mode), STY-BOLD (bold only for the parts the `bold` list of the file names, UX §7.3), DOC-GENERATED (the sheet, the names and the theme choices are the output of the file).
