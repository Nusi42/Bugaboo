# WP-14 — Exit review: screens

**Exit wave A · Folder / branch:** `bugaboo.win5` / `WIN5`
**Features:** WF-01…WF-06, ATT-01…ATT-07, NTF-01, NTF-02 (their screens).
**Source:** [the M4 exit review](exit-review.md), sections *The lead's notes on the screenshots*, *UX review*, and from the architecture section the web items named below.

## Goal

Every screen of M4 follows the approved UX concept on a computer and a phone, in all four themes, and the screenshots show it.

## Owns

`web/app/src/**` (screens), `web/packages/ui/**`, `design/theme.kdl`, the screenshot specs and the showcase, the M4 exit demo's pictures, `crates/bb-sample` only for the demo data (issue IDs, one issue per step, fresh data per theme).

## Tasks

1. **UX A1–A7** first (they apply to many screens): windows and questions pinned to the viewport, and the screenshots of windows taken at viewport size; one filled main button at the right end of every question; text buttons on the left edge; IDs, dates and times never wrap; the badge inside the narrow menu; demo data that holds together (IDs that read like WEB-142, the same issue in every theme, fresh data per theme); *Someone* for people the viewer may not see, as the glossary says.
2. **UX A8–A14** and **every "Must" and "Should" in UX B**, screen by screen. "Nice" items only where they are one line.
3. **The lead's notes**: the issue ID, the history refresh after a status change (find whether it is the product or the picture), the stray dash in the phone details line, *Reset to default* hidden at the organization level of workflows.
4. **Architecture, web (merge gate):** the workflow editor shares the layout editor's parts (`usedNow`, the kept-draft note, the locked editor, the remove dialog, screens and routes); the offers screens share one component; the "status by key, a new one by name" rule once; one `settingOf`; no raw query keys outside `shared/query-keys.ts` and no dead keys; upload start once; paste handler once; the "too big" limit read when shown; `file.no_room` by code; draft rules not copied from the server (an *Add* with a taken name says why); a workflow draft kept by a refused publish survives a reload (ADR-0019); *Try it* as a query; the file rows' Undo through `undoStore`; the status menu with `choiceLabelOf`; one notification list; no second names (`StatusGroupWord`, `changeSentence`, `LockedEditor`); the e2e tests take `PART_BYTES` and `MEGABYTE` from the generated files; a `Tabs` component in `ui` (M4-F81).
5. New screenshots of every changed screen in four themes, computer and phone, into `<work root>/screenshots/M4`. The builder looks at each in daylight and midnight against UX §1.1 and says what it saw.

## Depends on

`main` after WP-10. Works beside WP-12 and WP-13: WP-13 owns the wording of existing keys, WP-14 adds keys for new screen parts.
