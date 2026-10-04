# WP-06 — API skeleton: OpenAPI, generated TypeScript client, problem+json, rate limits

**Wave 2 · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** SEC-14, COMP-13, PRIV-01, UX-07 (error messages); ADR-0002, ADR-0008; follow-up M0-F01 (API part).

## Goal

One documented API surface. The browser only talks to it through a generated client; errors are problem+json with a stable code and a message key.

## Owns

`crates/bb-api/src/{api,openapi,errors,limits,cookies}*` (WP-04 owns the web/TLS parts), `web/packages/api-client/**` (new), the `gen api` step in xtask, checks `arch_errors.rs` (API part), `arch_api_client.rs`.

## Tasks

1. **OpenAPI (utoipa 6):** `/api/v1/...` router with the spec served at `/api/v1/openapi.json`; `cargo xtask gen api` writes the spec to `web/packages/api-client/openapi.json` and generates types with openapi-typescript; DOC-GENERATED covers both.
2. **Errors:** axum error type mapping `bb-core::ErrorCode` to problem+json (`type`, `title` = message key, `status`, `code`, request ID). No free-text error details reach the client; internal details go to the log only.
3. **Rate limits (SEC-14):** per IP and per account for sign-in-like routes, global per IP for the rest; limits from start-up config now, settings registry later; `Retry-After` header; plain-language message key.
4. **Cookies (COMP-13, PRIV-01):** only essential cookies (session, CSRF); a single cookie policy module that sets `__Host-`, `Secure`, `HttpOnly`, `SameSite=Strict/Lax` — no other code sets cookies (check). No consent banner needed; a short "cookies we use" list is generated for user docs.
5. **Browser client:** thin fetch wrapper using the generated types, CSRF header, problem+json → message key; TanStack Query integration hook. ARCH-API-CLIENT check: no `fetch(` to `/api` outside this package.

## Acceptance

- Contract test: spec on disk equals the served spec; TS types compile; an example endpoint round-trips through the client in a Vitest test.
- Rate-limit integration test returns 429 with `Retry-After`; cookie test asserts the attributes.
- All checks green.
