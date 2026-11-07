# WP-05 — The MCP server and how an AI app signs in

**Wave 2 · Folder / branch:** `bugaboo.win2` / `WIN2`
**Features:** AI-20, AI-21 (the tools offered through MCP), AI-22, AI-23; AI-24 in part (M7-Q3). ADR-0042, ADR-0013, ADR-0029 decisions 1 and 7, ADR-0025 decision 8, architecture §7 *MCP*; follow-ups M2-F61, M2-F65.

## Goal

An outside AI app (Claude Code, Claude Desktop, an IDE) connects to Bugaboo through MCP and works as the person who allowed it, never with more: every call goes through the one tool registry and the policy engine, answers are filtered per field, and the history says *"Ada Brandt via Claude Code"*. The app signs in with OAuth 2.1 and PKCE after the person allowed it, or with an access key; the person can see and take back every app.

## Owns

`crates/bb-mcp/**`, `crates/bb-auth/src/oauth*`, `crates/bb-app/src/auth/oauth*`, `crates/bb-store/src/oauth*`, `crates/bb-api/src/api/oauth*`, the routes `/mcp` and `/.well-known/oauth-authorization-server` and `/.well-known/oauth-protected-resource`, the consent step in `web/app/src/features/connect/**`, *Connected apps* in `web/app/src/features/my-settings/connected-apps*`, its migrations (numbers from the lead), the `mcp` rows of SEC-MATRIX, the register entry of `rmcp`.

## What is decided (ADR-0042, M7-Q3 and Q4 as recommended)

- **The server:** `/mcp` on the server's own port, streamable HTTP through `rmcp`. It lists the tools of the registry the actor's scope allows, with their descriptions and schemas, and the resources of AI-21 (an issue, a saved search) and its prompts. A tool call runs through the registry (WP-03): a tool that changes something runs only with *See and change*; every call is logged by the registry.
- **Signing in:** OAuth 2.1, authorization code with PKCE (S256 only), the metadata documents of the MCP authorization spec. An app with a return address on the person's own computer may register by itself; an app on a web address needs an administrator to add its address first. Access tokens live 15 minutes; refresh tokens turn over at each use, and an old one used again ends the whole grant. Every secret is stored as a hash. A grant holds a scope (*See only* or *See and change*) and, like a key, optionally a list of projects; what no key holds no grant holds.
- **An access key** works too (`Authorization: Bearer bb_…`), as the person narrowed by the key.
- **An agent** signs in with its agent key, or with OAuth for its app when its person allowed that app for the agent (the consent step names the agent).
- **The consent step:** a page of the browser app, *"Claude Code wants to work with your account"*, with the scope in plain words, the projects, *Allow* and *Cancel*; signing in first if needed. *Connected apps* under *My settings › Sign-in and security* lists each app with its scope, when it was allowed and last used, and *Take back*. *Sign out everywhere* also ends every grant.
- **Each MCP session is one run** (`RunStore`, WP-01's shape; WP-07 implements it): every change the session makes carries the run.
- **Rate limits** per grant and per person (architecture §7), with the answer of a limit as a sentence; refused calls in the security log.

## Tasks

1. **OAuth:** registration, authorization with the consent step, token, refresh, take-back, the metadata documents; tables of apps, grants and tokens (hashes), marked (DATA-MARKS); the jobs `oauth.clean-old-grants`.
2. **The actor from a token:** an access token, an access key or an agent key into the actor of WP-04 (*person via app* or *agent*), with the app's name.
3. **MCP:** `rmcp` over the registry: list tools, call a tool, resources, prompts; answers through the same safe types as the API (ARCH-API-OUT, M2-F61), filtered per field.
4. **Runs:** open a run when a session starts and close it at its end or after an idle time (a named constant), through the port.
5. **Screens:** the consent step and *Connected apps*, on computer and phone; pictures in four themes looked at against UX §1.1 and §7 before the push; the report names them.
6. **SEC-MATRIX:** the entry point `mcp` with a row for every tool, every role and both actor kinds (M2-F65); the API rows of the OAuth routes.

## Acceptance tests (write first)

- `ai_20_an_app_lists_only_the_tools_its_scope_allows`.
- `ai_20_an_mcp_call_as_a_contributor_never_shows_a_field_hidden_from_contributors` (the first step of the exit demo, on the server).
- `ai_22_a_code_without_its_pkce_proof_is_refused`.
- `ai_22_a_refresh_token_used_twice_ends_the_grant`.
- `ai_22_an_app_on_a_web_address_needs_an_administrator_first`.
- `ai_22_a_see_only_grant_cannot_call_change_status` (architecture §8, the security log row).
- `ai_22_sign_out_everywhere_ends_every_grant`.
- `ai_23_a_change_through_mcp_says_person_via_app_in_the_history`.
- `ai_23_an_access_key_works_as_the_person_narrowed_by_it`.
- `ai_38_an_mcp_session_is_one_run` (against the fake of `RunStore`).
- `ai_24_connected_apps_lists_an_app_and_take_back_ends_it` (browser).
- The MCP cases of TEST-PROMPT-INJECTION run through `/mcp`.
- SEC-MATRIX: the `mcp` entry point and the OAuth routes, both databases.

## Checks

SEC-MATRIX, ARCH-API-OUT, ARCH-AI-TOOLS, ARCH-HTTP-EDGE, SEC-SECRETS, SEC-COOKIES, SEC-HEADERS, DATA-MARKS, DATA-LOGS, DEP-REGISTER, PERF-N1, TEST-A11Y, THEME-ONLY, STY-PARTS, TXT-TERMS.

## Review

The SSOT/DRY/ODR gate of coding-standards §1: the tools only from the registry, the scope words of access keys reused, one narrowing for keys and grants, one way from a token to an actor. A security reviewer reads OAuth against the MCP authorization spec and OAuth 2.1 (PKCE, return addresses, refresh turnover, hashes). A UX reviewer looks at the pictures of the consent step and *Connected apps*.

## Depends on

Wave 1 merged (WP-02, WP-03, WP-04), WP-01 (ADR-0042, the port `RunStore`, the words). Merges after WP-07.
