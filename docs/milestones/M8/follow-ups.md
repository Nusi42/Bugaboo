# M8 follow-ups

Points found while building M8 that were accepted for later. Each names where it will be done. The IDs (`M8-F01` …) are what the review refers to. WP-01 uses F01 and up, WP-02 F10 and up, WP-03 F20 and up, WP-04 F30 and up, WP-05 F40 and up, WP-06 F50 and up, WP-07 F60 and up, WP-08 F70 and up, WP-09 F80 and up, WP-10 F90 and up, WP-11 F100 and up, WP-12 F110 and up; rows found by the lead outside a package take F200 and up.

## From WP-02 (a green full run)

WP-02 brought a full run from five failing browser tests to one (2026-11-11): Chromium opens one connection per page instead of two, the start probe reuses its connections, the limit of calls of AI apps and the crowd of the concurrent test count on the clock of the app, and five waits were fixed at their cause. The maintainer let it land with the rest open (2026-11-11).

| ID | What | Why later | Planned in |
|----|------|-----------|------------|
| M8-F10 | About one full run in two still fails one Chromium test at its first `page.goto` with `net::ERR_NO_BUFFER_SPACE`. It is not the connections in TIME_WAIT: with about 15,700 on the computer, 12,000 new connections still succeeded. Windows logged Tcpip events 4231 and 4266 (no ports left) on several days, once during a run | The bursts are shorter than a sample of two seconds; a finer look is needed | The lead: a network trace of Windows during one full run (the maintainer starts it as administrator), then the fix chosen from it, beside wave 1 |
| M8-F11 | WebKit opens about 47 secure connections per page that only shake hands and close; all requests go over one other connection. The WebKit part at the end of a full run peaks at about 1,050 connections in TIME_WAIT, so TEST-CONNECTIONS keeps its limit of 1,800 | Below 600 needs a choice: trust the test certificate in Windows, close such connections on the server without TIME_WAIT, or fewer WebKit pages at once | With M8-F10, from the same trace |
| M8-F12 | `cfg_08` of the quick search (an arrow key goes on from the search box, desktop) failed once: the saved searches and the issues come in one answer, and the focus is given in a `setTimeout` of `use-fresh-list.ts` | The order could not be made fixed in the test; the race may be in the app | With M8-F10 |
| M8-F13 | `iss_17` with three people on one lock (WebKit) once showed "ran out" instead of "taken over": the heartbeat may answer after a take-over (`edit-lock-session.ts`) | Product logic of the edit lock; not changed in a test package | With M8-F10 |
| M8-F14 | `qry_06 Save keeps a change of one's own search` (desktop) failed in the M7 exit runs but in no run of WP-02 | Not reproduced | With M8-F10, if it comes back |
