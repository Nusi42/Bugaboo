import { t } from "@bugaboo/messages";
import { type Browser, expect } from "@playwright/test";

import { lostNote, signedInPage, type ScreenLook } from "./crowd-pages.ts";
import { goodInFourThemes } from "./demo.ts";
import { savedNote } from "./issues.ts";
import { made, theTwo } from "./demo-m3-state.ts";
import {
  descriptionBox,
  expectToldWhoIsEditing,
  openEditing,
  saveDescriptionButton,
  startEditing,
  typeUnderLock,
} from "./description.ts";
import { sampleAdministrator } from "./sample.ts";
import { type Instance } from "./server.ts";

// The last step of the exit demo of M3 (`m3-exit-demo.spec.ts`): two people editing one description.

const FIRST_TEXT = "The first text, written while the second person watched.";
const SECOND_TEXT = "The second text, written after Edit anyway.";

/** Step 5: two people edit the description at once (the lock, Edit anyway, Compare and save). */
export async function twoPeopleEditTheDescription(
  browser: Browser,
  server: Instance,
  look: ScreenLook,
): Promise<void> {
  const first = theTwo().first;
  // Taking the editing over (Edit anyway) is for an administrator, a Project lead or the like.
  const second = sampleAdministrator();
  const holder = await signedInPage(browser, server, first, look);
  const other = await signedInPage(browser, server, second, look);
  await startEditing(holder, made.key);
  await typeUnderLock(holder, FIRST_TEXT);
  await goodInFourThemes(holder, "07-first-person-edits");
  // The second person is told who is editing, and cannot type.
  await openEditing(other, made.key);
  await expectToldWhoIsEditing(other, first.name);
  await goodInFourThemes(other, "08-second-person-is-told");
  // Edit anyway moves the lock; the second person saves their text.
  await other.getByRole("button", { name: t("editing.edit-anyway"), exact: true }).click();
  await descriptionBox(other).fill(SECOND_TEXT);
  await saveDescriptionButton(other).click();
  await expect(savedNote(other)).toBeVisible();
  // The first person's text is kept, and they compare and save it.
  await saveDescriptionButton(holder).click();
  await expect(lostNote(holder)).toBeVisible();
  await holder.getByRole("button", { name: t("issues.compare-and-save"), exact: true }).click();
  await expect(holder.getByText(SECOND_TEXT).first()).toBeVisible();
  await goodInFourThemes(holder, "09-compare-and-save");
  await holder.getByRole("button", { name: t("issues.compare-save-mine"), exact: true }).click();
  await expect(holder.getByText(FIRST_TEXT).first()).toBeVisible();
  await goodInFourThemes(holder, "10-the-text-is-kept");
}
