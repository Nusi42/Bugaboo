# ADR-0028: The message catalog is split into parts; the first page loads one

Status: **Accepted** · 2026-10-13

## Context

`messages/en.json` is the one source of every word (UX-07). The browser got all of it as one object, so every text of every screen was in the first page, and each new screen made the first page bigger: the catalog was about 22 KB of the 250 KB the first page may have (PERF-04, ADR-0009). The next package in line already pushed the first page over the budget through new texts alone.

## Decision

1. **`en.json` stays the one source, and the generator writes the parts.** `cargo xtask gen messages` writes one TypeScript file per part into `web/packages/messages/src/generated/`: `shared.ts` (the first page) and one file per folder of `web/app/src/features/`, plus `types.ts` (the type of every key, with no text in it) and `all.ts` (every part, for tests).
2. **One rule says where a key goes, and nobody keeps a list of keys.** Code names a key when it writes it as a string, or builds it from a template (`` `issue-field.${code}` `` or `"issue-field." + code`): a template names every key that starts with its text. Comments do not count. A key goes to the part of a feature when that feature is the only code that names it (the same reading TXT-CATALOG uses for "used"). It goes to `shared` when code outside the features names it, when two features do, and when no code names it: the server sends such a key, or a word of an enum makes it, and then any page may show it. The keys that the words of the enums make (error codes, `issue-field.`, `starting-role.`, permissions and so on, from one list that the generator and TXT-CATALOG share) are always `shared`, whatever the code says: the server sends them by key and shared code looks them up at run time. The one table in the generator lists the parts (`shared` and the feature folders); it stays a table, because the paths of generated files are known without reading the folders (META-COVERAGE counts them). A test and TXT-CATALOG compare it with the folders, so a new feature cannot be forgotten (its texts would stay on the first page until it is added). A new key therefore starts on the first page and moves to its area as soon as the area's code names it; the diff of the generated files shows it.
3. **`t()` stays synchronous and keeps its types.** Every part registers its texts when its file is loaded (`registry.ts`). The entry of a feature (`index.ts`) imports its part as its first import, so the part lands in the chunk of that feature and is evaluated before the feature runs. Routes load a feature with `import()` before they render it, so the texts are there; a screen that sits behind `OnlyWith` has a route `loader` (`loadsScreen`) that loads it first, because the router cannot see a lazy component inside another one. `isMessageKey` is true for loaded keys only, which is why the code that looks keys up by a string lives in the area whose part holds them, or the keys are on the first page.
   **A text that is not loaded** is a defect of the build, and a person never sees a key. While the app is built for development (`import.meta.env.DEV`, which vitest sets too) `t()` **throws** with the key in the message, so the defect is found at once. In production `t()` writes `console.error` once per key, naming the key, and returns an empty text, so the rest of the page keeps working. The end-to-end tests fail a test on any `console.error` of the app (the shared fixture; the browser's own "Failed to load resource" lines for refused requests do not count), so the defect is still found before release. The vitest tests cannot find a part that an area forgot to import, because they load every part; TXT-CATALOG finds it.
4. **Outside the browser app everything is loaded.** `@bugaboo/messages` resolves to an entry that loads every part for Node (unit tests and end-to-end tests); the browser build resolves to the plain entry.
5. **Precompiling the messages at build time is not done now.** It would let the first page drop the ICU parser (about 57 KB of source, some 8 KB after Brotli), but it needs the ICU syntax tree written by the generator and an alias of the parser in the build and not in the tests. Splitting the catalog gives the room that was asked for; the parser can go later, as its own decision.

## Consequences

- The first page carries the texts of the shell, sign-in, setup and errors shown anywhere, plus the texts of the features that the first page itself loads. A feature that is loaded lazily brings its texts in its own chunk; the texts of the rest of M3 and beyond land there too.
- `cargo xtask gen messages` now reads the code of `web/` as well as the catalog. DOC-GENERATED fails when a change of code moves a key, and `gen messages` fixes it.
- A feature that the first page loads statically (today `account`, `shell`, `sign-in`, `home`) keeps its texts on the first page. Making such a feature lazy moves its texts with it.
- A text that two lazy features show is on the first page. That is cheap for common words and a reason to name such a text once in shared code.

## Alternatives considered

- **A table from the first part of a key (its area) to a part.** Simple, but a key shown by two areas (a `projects.` text that the issues screens also show) cannot be placed, and the table drifts from the code.
- **Load a part asynchronously from `t()`.** Makes `t()` asynchronous at every call site. Rejected.
- **One chunk per area chosen by hand in the build configuration.** A second list of areas that nothing checks. Rejected.

## Enforced by

DOC-GENERATED (the parts are the output of the catalog and the code), TXT-CATALOG (every feature folder has a part, the entry of every feature with texts imports its part as its first import statement, in code and not in a comment, and nothing else does), the end-to-end fixture (a `console.error` of the app fails the test), PERF-BUNDLE (the first page stays within its budget), LINT-TYPES (every key keeps its type), and the tests in `web/packages/messages` (every key is in exactly one part, a key is found once its part is loaded, a missing text fails).
