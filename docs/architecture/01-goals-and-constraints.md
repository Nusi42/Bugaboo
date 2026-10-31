# Goals and constraints

## Quality goals

Ranked. When two goals conflict, the higher one wins, and the trade-off is recorded in a decision record (ADR).

| Rank | Goal | What it means for the architecture |
|------|------|------------------------------------|
| 1 | **Security, secure by default** | One authorization path for every way in (UI, API, search, MCP, assistant, exports, notifications). Deny by default. Hidden data never leaves the server, not even as a count or an error-message difference. |
| 2 | **UX** | Plain, fast, works for people less technical than average. Common actions feel under 100 ms; the server sends only what a screen shows. |
| 3 | **AI integration** | The assistant and outside AI apps (MCP) act as the person, with that person's permissions, through the same code as the UI. AI proposes; people decide. |
| 4 | **Software and design architecture** | SSOT, DRY and ODR are mandatory. Clear module boundaries, testable core, generated code instead of copied code. |
| 5 | **Feature completeness** | The feature list, with no architectural dead ends for later features. |
| 6 | **Ease of installation** | One binary, an embedded database, a setup wizard. |

Across all of them: **privacy by design** (GDPR) and **EU product law** (CRA, AI Act, Data Act, NIS2) are constraints, not features. See [compliance.md](../compliance.md).

## Constraints

### Technology

- **Server:** Rust, latest stable (1.97 at time of writing), edition 2024. `unsafe` is forbidden unless an ADR allows it.
- **Browser app:** TypeScript 7 (the native compiler) and React 19.
- **Exact versions:** see [Tech stack](09-tech-stack.md). Versions are pinned through lockfiles and updated on purpose.

### Platforms

- **Server:** Windows 10/11 and Windows Server 2022+ (x64, ARM64); Linux (x64, ARM64, any distribution, static build); WSL 2 uses the Linux build.
- **Browsers:** the last two versions of Chrome, Edge, Firefox and Safari, on desktop and phone.

### Data and scale

- **Databases:** SQLite, built in, by default; PostgreSQL 16 or newer for larger installations.
- **Scale target for 1.0:** about 500 people, 1 million issues and 200 people working at the same time on one installation, on a 4-core server with 8 GB of memory.
- **One organization per installation.** Projects separate teams inside it.

### Distribution

- Bugaboo may be **sold** (the customer runs it) or **served** (we run it). Both must work from the same code (DIST-01; review: checked at every milestone review).
- **No telemetry** and no outside requests unless an administrator configures them (mail, AI provider, single sign-on, update check).

### Licenses of third-party parts

Bugaboo itself is licensed under the GNU Affero General Public License v3.0 or later (ADR-0036), and the maintainer also offers it under other terms. Nothing from the GPL family may therefore be in it, and every part we ship must be under one of these licenses (DEP-LICENSES):

| Allowed in the shipped binary | Not allowed |
|-------------------------------|-------------|
| MIT, Apache-2.0 (also with LLVM exception), BSD-2/3-Clause, ISC, Zlib, Unicode-3.0, MIT-0, 0BSD, CC0, Unlicense, BSL-1.0, CDLA-Permissive-2.0 (data), OFL-1.1 (fonts), MPL-2.0 (file-level, unmodified) | GPL, AGPL, LGPL (static linking), SSPL, BUSL, "non-commercial" licenses |

The exact allow list lives in `deny.toml` (the single source) and is enforced by DEP-LICENSES (coding-standards §5).

### People and process

- Implementation is done by AI agents in milestones, guided by these documents. Everything an agent needs must be written down (review: checked at every milestone review), and every rule that can be checked by a machine is checked by a hook.
