# ADR-0036: License: AGPL-3.0-or-later, other terms from the maintainer, a contributor agreement

Status: **Accepted** · 2026-10-31

## Context

Bugaboo is about to be published. The license of the code was open (the feature list said "decide later"). What is wanted:

- Anyone may run and change Bugaboo, also in a closed setting for their own use.
- Whoever distributes a changed version, or offers it to others over a network, must make its source available, so the maintainer can read and merge it.
- The maintainer can also offer Bugaboo under other terms, for example an edition with additions that are not published.

## Decision

1. The code of Bugaboo is licensed under the **GNU Affero General Public License v3.0 or later** (SPDX `AGPL-3.0-or-later`). `LICENSE` holds the official text. The identifier is in `Cargo.toml` (`[workspace.package]`, which every crate inherits) and in every `package.json` of the project.
2. The maintainer offers Bugaboo under other terms on request. To be able to do that for the whole code, every contribution comes with a **contributor license agreement** ([`CONTRIBUTING.md`](../../CONTRIBUTING.md)): contributors keep their copyright and grant the maintainer a perpetual, irrevocable, royalty-free license to use, sublicense and relicense the contribution under any terms, plus a patent license. It is accepted in the pull request with a comment.
3. **No dependency from the GPL family** (GPL, LGPL, AGPL, in any version) goes into Bugaboo: code under someone else's GPL could not be offered under other terms. The allow list in `deny.toml` names none of them, so `cargo deny` refuses them for Rust, and DEP-LICENSES refuses them for npm packages, also those that only run on a developer machine. A package that offers a choice (`MIT OR GPL-3.0-only`) is fine when the other side is on the allow list.
4. Parts that third parties build on (a client library, a plug-in kit, examples) may later be under the MIT license, so that using them does not carry the AGPL into other programs. None exists as a separate product today; each such part needs its own decision record.
5. Until a lawyer has reviewed the agreement, nothing commercial is offered.

## Consequences

- Every new crate or package carries the license (`license.workspace = true`, or `"license": "AGPL-3.0-or-later"` in `package.json`).
- A change of the allow list or of the license of the project needs a new decision record.
- A pull request is merged only after its author has accepted the agreement (review).

## Alternatives considered

- **MIT or Apache-2.0 for everything:** anyone could close a changed version and keep it, and the maintainer could not read or merge it. Rejected.
- **GPL-3.0:** does not cover use over a network, which is how Bugaboo is run. Rejected.
- **A source-available license (BUSL, SSPL):** not open source, and the allow list refuses such licenses in dependencies for the same reasons. Rejected.

## Enforced by

DEP-LICENSES (the allow list has no GPL family, npm included, with cargo deny and `deny.toml`), review for the agreement in pull requests.
