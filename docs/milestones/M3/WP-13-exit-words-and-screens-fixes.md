# WP-13 — Exit review: words, screens and accessibility fixes

**Wave 5 (exit fixes) · Folder / branch:** `bugaboo.win3` / `WIN3`
**Features:** ISS-01, ISS-11, ISS-17 (browser), UX-04, CF-01, CFG-01, ISS-09; UX design §3.2, §8, §9; ADR-0028.
**Source:** the M3 exit UX, accessibility and words review, copied below. Overlaps M3-F56 (phone bar), which this package closes.

## Goal

Nobody gets stuck: every refusal says what happened and offers the way out that exists on that screen. Each meaning has one text and one message key. The issue page, side panel, tablet sheet and phone agree, and the screen reader hears what a sighted person sees.

## Owns

The message catalog sources and generated parts (ADR-0028), `docs/glossary.md` (words only), `web/app/src/features/{issues,history,layouts,bulk,labels,fields,related-issues,sub-issues}/**` (screens and texts), `web/packages/ui/**`, `design/theme.kdl` (touch targets), the server's `error.field.*` refusal carrying the field (the smallest change in `bb-core`/`bb-api` for that). WP-14 owns `new-issue-form.ts`'s default priority and the generated edit parts; coordinate.

## Tasks

1. All **must-fix** items 1–5 below, each with an e2e or unit test written first.
2. All **should-fix** items, except "Make vs Create": that waits for the maintainer's answer (leave those texts unchanged).
3. The whole **SSOT/DRY/ODR (texts)** section: one key per meaning (shared keys in shared code, ADR-0028), one label for "type an issue ID", one "required" wording (*Must be filled in* per the glossary, or the glossary changes with it), one bulk preview, one "who changed it", lists joined with `Intl.ListFormat` from one shared helper, `InPlaceText`'s text from the catalog.
4. The **nits**.
5. New follow-ups from M3-F140 up.

## Checks

TXT-LITERAL, TXT-TERMS and THEME-ONLY stay green; accessibility checks in four themes on the touched screens.

## Depends on

M3 WP-11.

## Findings (exit UX, accessibility and words review)

## Must-fix

1. Comment editor dead end (`web/app/src/features/history/CommentEditor.tsx:139-143`, `CommentEntry.tsx`): when the comment is locked (or locking fails) the text disappears, only "Someone else is editing this comment." stays, no Close/Cancel, Edit still shown and does nothing; text back only after reload. Fix: keep showing the comment; same note as the description ("{name} is editing this comment · since {since}" + Close; reuse `issues.is-editing` with the thing as a parameter); hide Edit while the editor is open.
2. `error.edit.version-changed`, `error.edit.lock-lost`, `error.edit.taken-over` all say "Use Compare and save…", but comments (`CommentEditor.tsx:117`) and the layout editor (`layouts/SaveBar.tsx:43`) show them and have no such button (layout editor also shows `layouts.lost` → two instructions). Fix: general messages name no button ("Not saved yet: somebody changed this after you started. Your text is still here."); each screen shows its own way out.
3. Required own fields not marked on the new-issue form (`fields/box-words.ts`, `issues/NewIssueForm.tsx:54`); only title has hard-coded "(required)" in `issues.title-required`; after Create, `error.field.required` "This field has to be filled in." appears at the bottom without naming the field (same for every `error.field.*`). Fix: one message "{field} (required)" for every required field incl. title; server refusal carries the field and is shown at that field ("Fill in Severity.").
4. Short-value conflict note never says it was not saved (`issues/ConflictNote.tsx`, `conflict.sentence`): "Not saved: {who} changed {changes} {ago}." (UX §9).
5. `layouts.lost`: "Ask for the lock again to save them." uses "lock", button says Try again, no reason. Fix: "Not saved yet: {name} took over, or your editing time ran out. Your changes are kept here. Choose Try again to save them."

## Should-fix

- Names mid-sentence use start-of-sentence form ("Ask A Team member to finish", "Keep Someone's"): `DescriptionNotes.tsx` (personName), `use-issue-changes.ts:39` (whoChanged), `layouts/use-layout-lock.ts:31` → `personNameInside`; for Someone/role say "Keep their change".
- Side panel / sheet / phone: `IssueFrame.tsx` SidePanel (heading + "Close the panel") plus `IssueHeader.tsx:58-62` copy button + second Close → two closes, ID twice; phone and upright tablet say "Close the panel" (UX §3.2 wants Back). Tablet sheet name just "Issue" (`IssuesWithPanel.tsx:52`, `issues.sheet`). Fix: one close; "Back to list" (`focus.back`) on phone/upright tablet; name sheet with `issues.title-of`.
- Phone bar (`IssueFrame.tsx` focusStatus/goTo; overlaps M3-F56): Status focuses the first select on page (Priority when status read-only); Comment only scrolls. Reuse ChoiceFact's scoped target; focus into comment box.
- Copy button (`ui/CopyButton.tsx`, in `IssueHeader.tsx`): name "WEB-142, copy the address"; announce "Address copied" via status.
- Issue list rows (`IssueRow.tsx:29-30`): type icon only and hidden from screen readers (its comment wrongly says the name is next to it); title link < 44 px on phone (`issue-row.title`, `link` parts in theme.kdl); "N issues are ticked" (`Selected.tsx`) not announced (UX §8).
- Buttons say what they do: `bulk.apply` "Apply" (also delete) → "Change 12 issues"/"Delete 12 issues"; `issues.side-panel-view` → "Open in the side panel"; plain Edit/Delete → "Edit the description", "Edit your comment".
- Throwing away: `issues.draft-drop` "Start again" → "Throw the draft away"; `layouts.discard` → reuse `layouts.draft-drop` "Throw my changes away".
- Relation sentences with noun-like kinds (`related-issues.added`, `history.relation-added`, `bulk.line-relation`): "WEB-142 now duplicate of WEB-9" → "Relation added: WEB-142 · duplicate of · WEB-9" or glossary forms with "is".
- `bulk.delete-text` "The issues are deleted" → "The issues go to the trash. You can undo this for a short while."
- `layouts.saving/saved/unsaved` → glossary `save.*` (Saving… / Saved / Not saved yet).
- `layouts.limited-page`/`limited-form` "{role} can't see {field}: Field permissions" → "{role} cannot see {field} because of its field permissions." + a real link.
- `layouts.use-inherited*` → glossary "Reset to default" or "Use the layout of {place} again".
- Make vs Create: issues use "Create" (`issues.create`, `create-another`, `create-first`); `shortcut.new-issue`, the permission and 34 other forms say "Make …". One verb + glossary entry. (MAINTAINER DECISION)
- Combine: `labels.combine-title` "Combine into another", `labels.combine-into` vs glossary/fields "Combine with another choice".
- Skip reasons: `bulk-skip.not-found` → "It was deleted, or you can no longer see it"; `bulk-skip.limit-reached` → "It has as many relations as an issue can hold"; `bulk-skip.not-valid` → name the rule; `bulk-skip.field-missing` → "It has a value in a field the other project does not use".
- `issues.editing-elsewhere`: add a way out without Edit anyway: "Save or close it there. If that window is gone, wait a minute and try again."

## SSOT/DRY/ODR (texts) — blocking under §1

Same meaning, two keys: `issue-ref.status-*` = `status-group.*` (`IssueRefLine.tsx:10`); `issues.someone` = `projects.person-someone` (`DescriptionNotes.tsx:27` bypasses `someoneName`); `layouts.edit-anyway`/`issues.edit-anyway`; `layouts.ask-finish`/`issues.ask-to-finish`; `layouts.try-again`/`common.try-again`; `layouts.saving/saved/unsaved`/`save.*`; `issues.compare-title`/`issues.compare-and-save`; `compare-keep-mine`/`compare-mine`; `compare-keep-theirs`/`compare-theirs`; `comments.save`, `labels.save`/`common.save`; `comments.cancel`, `related-issues.cancel`/`common.cancel`; `comments.undo`, `sub-issues.undo`, `related-issues.undo`/`common.undo`; `issues.history`/`history.title`; `bulk.field-status`, `bulk.field-priority`/`issue-field.*`; `sub-issues.part-of`/`issue-field.parent`; `issue-types.saved`, `relation-kinds.saved`, `labels.saved`/`save.saved`. (ADR-0028: text shown by two lazy features is named once in shared code.)
Four labels for "type an issue ID": `sub-issues.field`, `related-issues.issue-field`, `bulk.field-other`, `fields.pick-issue-key` (different examples WEB-142/WEB-12). Relation kind: `bulk.field-kind` vs `related-issues.kind-field`.
Five "required": `validation.required`, `error.field.required`, `fields.field-required`, `layouts.field-required`, `issues.title-required` (glossary: Must be filled in).
Bulk preview is one component per glossary but `labels.preview-summary/preview-nothing/preview-label` copy `bulk.summary/nothing/preview-label`; `bulk.result-done` ≈ `bulk.finished`.
Code: "who changed it" twice (`use-issue-changes.ts:39-43`, `conflict-text.ts:47-51`); lists joined with literal ", " / " · " in `IssueRow.tsx:32`, `FollowersBlock.tsx:70`, `issue-values.ts:126`, `UsageSection.tsx:74`, `FieldPage.tsx:47`, `FieldsPage.tsx:20` — use `listOf` (Intl.ListFormat) from `conflict-text.ts` moved to shared; `InPlaceText` builds `${changeLabel}: ${value}` in code instead of the catalog.

## Nits

`layouts.saving`/`layouts.taking` space before "…"; `conflict.sentence` stray space when no time ("…to Critical ."); `issues.compare-keep` → "Which text to keep for paragraph {n}", order of choices vs columns; `issues.draft-kept` says "window" (it's a form); `issues.lost-ran-out` → say why ("you were away for a while"); `IssueBar` uses nav landmark → group/toolbar; `fields.used-in-projects` 0 → "No project you can see uses it."; `layouts.delete-held` unclear; mixed can't/cannot; many verbs for remove (Take away/Take out/Take off/Remove); "Create and start another" disabled without saying why; bulk dialog text repeats `bulk.bar`.

Good: catalog follows glossary closely; description notes always give a way on; preview sentence matches glossary; drag has buttons; problems announced at once, hints wait.
