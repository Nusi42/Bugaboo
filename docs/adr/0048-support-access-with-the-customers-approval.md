# ADR-0048: Support access with the customer's approval

Status: **Proposed** · 2026-11-18 · refines DIST-03, [ADR-0012](0012-sold-and-served.md) and boundary 4 of [architecture §12](../architecture/12-security.md); open questions M9-Q15 and M9-Q20 of [the M9 plan](../milestones/M9/README.md)

## Context

DIST-03 says that our staff have no access to customer data by default, and that emergency access happens only with the customer's approval, time-limited, and recorded in the customer's own security log. Architecture §6 sketches it for the served model: "an operator requests access; the customer's administrator approves in their own Bugaboo".

The served model and its control plane come in M14. But the need comes earlier: a self-hosted customer who asks us for help with their installation makes us their processor for that support case (compliance.md §5), and the same rules apply. What can be built now is the customer's side: a way for an administrator to let one named supporter in, for a short time, with limited rights, seen by everybody who reads the security log, and ended at any moment.

## Decision

1. **Nothing by default.** Bugaboo has no built-in account, key or back door for us or anybody else, in either model. Enforced by: SEC-MATRIX (no route answers without a session or key that the organization made), review (no account is made outside *Support access* or an invitation).
2. ***Support access* is a time-limited account the administrator makes** (M9-Q15, recommendation). In *Organization settings › System › Support access* an administrator with *Change organization settings* names the supporter (a name and an e-mail address), a reason, a role (any role that holds no permission the administrator lacks; the start is *Viewer*), the projects it counts in, and an end (at most 72 hours, `support-access.longest`; the start is 8 hours). Bugaboo shows a one-time link, good for one hour, which the administrator passes on. Enforced by: ARCH-PERMS, ARCH-SETTINGS, SEC-MATRIX.
3. **The supporter signs in with two steps.** Opening the link sets a password and a code app or passkey before anything else; there is no access key, no AI app grant and no agent for such an account. Enforced by: SEC-MATRIX, ARCH-AI-TOOLS.
4. **Everything is in the customer's security log.** The making, each sign-in, every request of the supporter (not only changes: route and record, never the content), and the end. The requests are appended one after another like every write (M1-F23), and their number is bounded by the API's limit per client (`rate_limit_per_minute`), so a supporter cannot slow the others more than any client can. People see a support account as *"Sam Lee (support access until 18:00)"* wherever a person is named. Enforced by: ARCH-EVENTS, ARCH-PERSON-REF.
5. **It ends by itself and on demand.** At its end, or when any administrator chooses *End now*, the sessions end at once and the account is removed as in *Remove a person* (ADR-0044), keeping its security-log entries. The record of the access (supporter's name, reason, role, places, start, end, maker) is kept for the keep period of the security log, then deleted by that keep rule (ADR-0045), so a log entry can always be read with who and why. A job checks every minute so no session outlives its end by more than that. Enforced by: SEC-MATRIX (a request one minute after the end is refused), ARCH-EVENTS.
6. **The served model later uses the same door.** In M14 the control plane may *ask* for support access through the operator API (DIST-02); the customer's administrator still makes it here, under the same rules. The control plane never holds a way in of its own. Enforced by: review (M14 builds on this record and does not add a second way).

## Consequences

- Support for self-hosted customers has a safe, recorded way from the first release.
- Every support case is visible to the customer afterwards, request by request.
- The supporter's work is limited to what the administrator chose, and Bugaboo, not trust, keeps it there.

## Alternatives considered

- **A shared support password or a built-in support account:** a back door. Rejected.
- **Screen sharing only:** works and needs no feature, but leaves no record in Bugaboo and lets the supporter see whatever is on the screen. Kept as the customer's own choice, outside Bugaboo.
- **Wait for M14:** DIST-03 is an MVP feature, and support for self-hosted customers starts with the first release. Rejected.

## Enforced by

SEC-MATRIX, ARCH-PERMS, ARCH-SETTINGS, ARCH-AI-TOOLS, ARCH-EVENTS, ARCH-PERSON-REF; review (no second way in).
