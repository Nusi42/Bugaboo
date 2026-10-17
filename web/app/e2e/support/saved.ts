import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { detailRow, partIn } from "./parts.ts";

/** The word *Saved*, at the start of a note. */
const saved = new RegExp(`^${t("save.saved")}`, "u");

/**
 * The *Saved* note under the title of the issue page: the one for a change that has no row in the
 * Details (the title, the status).
 */
export function issueNote(page: Page) {
  return partIn(page, "issue-note").getByText(saved);
}

/** The *Saved* note in the row of a field of the Details, found by the name of the field. */
export function rowSaved(page: Page, name: string) {
  return detailRow(page, name).getByText(saved);
}

/**
 * The first *Saved* note anywhere on a page: of a description, a comment, a row or the title. A
 * test that knows which change it waits for names it (`issueNote`, `rowSaved`), so that the note of
 * another change cannot satisfy it; this one is for a page where only one change is made.
 */
export function savedNote(page: Page) {
  return page.getByText(saved).first();
}
