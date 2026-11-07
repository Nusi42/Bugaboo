# ADR-0042: The MCP server and how AI apps sign in

Status: **Accepted** · 2026-11-07 · refines [ADR-0013](0013-ai.md), [ADR-0025](0025-roles-groups-and-assignments.md) (decision 8) and [ADR-0029](0029-ai-agents.md) (decisions 1 and 7); decisions M7-Q3 and M7-Q4

## Context

M7 opens Bugaboo to outside AI apps (Claude Code, Claude Desktop, an IDE) through MCP (AI-20 to AI-23). An app acts for a person (ADR-0029, decision 1) and an agent signs in with its own key or through its app (ADR-0029, decision 2). The MCP authorization spec asks for OAuth 2.1 with PKCE and the metadata documents that let an app find the server's sign-in by itself. Architecture §7 planned an authorization server inside Bugaboo with an allow list kept by administrators, but did not say how long a token lives, what happens to a stolen refresh token, which apps may register, what a person sees before an app may act for them, or how a person takes an app's access back. The maintainer accepted the recommendations of M7-Q3 (the consent step and a plain *Connected apps* list in M7) and M7-Q4 (which apps may register) on 2026-11-07. This record writes it down for WP-05 and WP-09.

## Decision

### 1. One MCP server inside Bugaboo

- The MCP server is part of the one program (ADR-0001), at `/mcp` on the server's own port, over the streamable HTTP transport, built with `rmcp`. It adds no listener and no port (ADR-0023).
- `bb-mcp` turns the one tool registry of `bb-app` into MCP and depends on `bb-app` only (architecture §4). Every call runs a tool of the registry as the actor, through `bb-policy`; there is no second rule set (ADR-0013, ADR-0029 decision 1). Enforced by: ARCH-AI-TOOLS, ARCH-DEPS.
- Answers leave through the same safe types as the API (`Authorized<T>` with its field mask, `PersonRef`), so a hidden field never reaches an app (AI-23, M2-F61). Enforced by: ARCH-API-OUT, SEC-MATRIX (the `mcp` rows).

### 2. Two ways in: OAuth or an access key

- **OAuth 2.1** (decision 3) is the way for an app a person connects by signing in.
- **An access key** is the other way in, for an app that cannot sign in and for agents (`Authorization: Bearer bb_…`, an agent key for an agent, ADR-0029 decision 2). It counts at `/mcp` exactly as at `/api/v1`: the person's evaluation narrowed by the key's choice and projects (ADR-0025, decision 8).
- A request with neither is answered `401` with the `WWW-Authenticate` header that names the metadata of decision 3, as the MCP authorization spec asks.

### 3. The authorization server inside Bugaboo

- **Authorization code with PKCE**, the method `S256` only (`plain` is refused). No other grant type: no password grant, no implicit flow, no client credentials.
- **Metadata:** the protected-resource metadata (RFC 9728) at `/.well-known/oauth-protected-resource` and the authorization-server metadata (RFC 8414) at `/.well-known/oauth-authorization-server`, as the MCP authorization spec asks. A token is issued for one resource, the server's `/mcp` (the `resource` parameter, RFC 8707), and refused anywhere else, `/api/v1` included.
- **Registration (M7-Q4):** an app registers itself (dynamic client registration, RFC 7591) when **every** return address it names is on the person's own computer: `http://127.0.0.1`, `http://[::1]` or `http://localhost`, on any port (RFC 8252), as Claude Code and Claude Desktop use. An app whose return address is a web address (`https://…`) may register only when an administrator added that address under *Organization settings › AI* first (*Apps that may sign in*). Any other address, and a web address not on the list, is refused at registration and again at every sign-in; the return address of a sign-in has to be one the app registered, compared as a whole string. Enforced by: SEC-MATRIX (the OAuth routes) and the tests `ai_22_an_app_on_a_web_address_needs_an_administrator_first` and `ai_22_a_code_without_its_pkce_proof_is_refused` (WP-05).
- **Codes and tokens:**
  - an authorization code works once and for 60 seconds, and only with its PKCE proof and the return address it was given for; a code used a second time ends the grant it made;
  - an **access token** lives **15 minutes**;
  - a **refresh token turns over at each use**: the answer carries a new one and the old one is dead. **An old refresh token used again ends the whole grant** (its access token and every refresh token), because then two parties hold it; the event is in the security log. A refresh token that is not used for 30 days runs out;
  - every code, token and client secret is a random 256-bit secret, **stored only as a hash** (SHA-256 with a domain of its own, like a session, architecture §7), and never written to a log (DATA-LOGS).
- **Scopes:** an app asks for *See only* or *See and change*, the words of access keys (ADR-0025, decision 8); *Everything I can do* and single permissions are not offered to apps. Like a key, a grant may also hold a list of **projects**, and then counts only in them and their sub-projects. A scope **only narrows the person**: at every call the grant's permissions are the person's evaluation at that place intersected with the scope, exactly as for an access key, and **what no key holds no grant holds** (`NOT_FOR_KEYS`). A grant never reaches the person's own sign-in data. Enforced by: SEC-MATRIX (the `mcp` rows: a grant with *See only*, one with *See and change*), and the property test `ai_22_a_grant_can_do_only_what_its_person_can_and_its_scope_allows` (WP-05).
- **Who a grant follows:** a grant follows its person like a key: a role taken away counts from the next call; while the person is asked for a second step a grant is refused like a key (ADR-0025, decision 7); a person who is switched off or removed loses every grant.

### 4. The consent step and *Connected apps* (M7-Q3)

- **The consent step:** before a grant is made, the person, signed in in the browser, sees one page: *"Claude Code wants to work with your account"*, what it may do (*See only* or *See and change*, the scope it asked for, which the person may narrow to *See only*), optionally the projects, its return address (*on this computer* for a loopback address), that the name is the app's own and Bugaboo cannot check it, and that it never does more than the person. *Allow* makes the grant; *Cancel* sends the app away with `access_denied`. The page is asked for every new grant; a remembered consent per app is AI-24 (M12).
- **An agent through its app:** an agent signs in with its agent key, or through OAuth when its person allowed that app **for the agent**: the consent step then names the agent (*"Claude Code wants to work as Fixer, agent of Ada Brandt"*), and the grant belongs to the agent, narrowed by its work and limits (ADR-0029, decisions 2 and 9).
- **Connected apps** (*My settings › Sign-in and security*): a plain list of the person's grants, each with the app's name, what it may do, when it was allowed and when it was last used, and **Take back**, which ends the grant at once (its tokens are dead at the next call). The details of each app's use stay in AI-24 (M12).
- ***Sign out everywhere* ends every grant** as it ends access keys (ADR-0025, decision 8), in the same change, each with a security-log entry. A password change leaves grants as it leaves keys. Enforced by: the test `ai_22_sign_out_everywhere_ends_every_grant` (WP-05).
- **Administrators can switch AI apps off** for the organization (a setting under *AI*, ADR-0013 amendment M7): no app registers or signs in, and every grant is refused at its next call, but none is deleted, so switching on again brings them back. AI switched off for the organization does the same.

### 5. Each MCP session is one run

- An MCP session (from `initialize` to its end, or 30 minutes without a call) is **one run** (ADR-0029, decision 7): its changes are listed together and undone in one step (AI-38), through the port of runs (`RunStore`, architecture §4).
- Every event of a call names the person, the channel `mcp` and the app: the history says *"Ada Brandt via Claude Code"* (AI-23); an agent's change says *"Fixer, agent of Ada Brandt"* (ADR-0029, decision 2). Text written through MCP carries *Written by an AI app* or *Written by an agent* (COMP-09).

### 6. Limits and the security log

- **Rate limits per grant and per person** (a key counts as its own grant): at most 120 calls a minute for one grant and 240 a minute for one person over all their grants and keys; a call over the limit is answered `429` with the time to wait, and nothing of it is done. Registration and the token endpoint count failed tries per network like sign-in (architecture §7, *Limits and abuse*). The numbers are named constants of WP-05. Enforced by: the test `ai_23_a_grant_over_its_limit_waits_and_changes_nothing` (WP-05).
- **The security log** has an entry for every registration, every grant made (*Allow*), every renewal that ends a grant (an old refresh token used again), every *Take back*, every grant ended by *Sign out everywhere*, and every refused sign-in of an app (an unknown return address, a code without its proof). A plain renewal of a token is not logged one by one: the grant's *last used* says it. Enforced by: ARCH-EVENTS.

### 7. Stored as personal data

A grant (person, app, scope, when allowed, last used) and a registered app (name, return addresses) are personal data of the person; the hashes are secrets. The marks are in architecture §12, *Personal data in M7*. Enforced by: DATA-MARKS.

## Consequences

- New tables for registered apps, grants, codes and refresh tokens, each with its marks; a daily job `oauth.clean-old-grants` deletes codes and refresh tokens that ran out and grants that were taken back (ADR-0017, amendment M7).
- New routes outside `/api/v1`: `/mcp`, the two metadata documents, registration, the authorization page (the consent step in the browser app) and the token endpoint; SEC-MATRIX gets the entry point `mcp`.
- The glossary gains *Allow*, *Take back*, *Grant* (internal) and the app sense of *See only* and *See and change*; *Connected apps* is in M7 in its plain form, and AI-24 keeps what is more.
- The threats are in architecture §12, *Assistant, AI apps and agents (M7)*.

## Alternatives considered

- **An outside authorization server (an identity provider of the customer):** the MCP spec allows it, but a self-hosted Bugaboo would then need one to run, and the scopes would live in two places. Rejected for M7; company sign-in (OIDC) can sign the person in on the consent page.
- **Open registration for every return address:** any web page could register as *"Claude"* and collect grants through a link. Rejected (M7-Q4): web addresses need an administrator.
- **Only access keys, no OAuth:** a person would copy a long secret into every app, and the MCP apps expect OAuth. Rejected; keys stay as the second way.
- **Long access tokens without refresh:** a stolen token would work for days. Rejected; 15 minutes and rotation make a theft short and visible.
- **A remembered consent per app in M7:** more screens and rules than M7 needs. Moved to AI-24 (M12, M7-Q3).

## Enforced by

SEC-MATRIX (the `mcp` rows), ARCH-API-OUT, ARCH-AI-TOOLS, ARCH-DEPS, ARCH-EVENTS, DATA-MARKS, DATA-LOGS (see `tools/checks/catalog.toml`), and the tests named above and in WP-05.

<!-- COMP-10 checked: signing in and scopes rank, evaluate or assign nobody; an app does what its person tells it, as ADR-0029 decision 9 says. -->
