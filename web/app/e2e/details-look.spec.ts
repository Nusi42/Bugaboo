import { t } from "@bugaboo/messages";
import type { Locator } from "@playwright/test";

import { ada, bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import {
  factorIssues,
  factorRow,
  openIssue,
  prepareTimeFactor,
  setFactorAs,
} from "./support/time-factor.ts";

// The look of rows of the Details (UX §4.4, §7.3, R1), on a computer and a phone: words that take
// two lines keep *Change* a line under them with no hole between, and a box for a length says *Add a
// length* in the face of the text (a time value is in the code face, its hint is not).

test.use({ preparation: { run: prepareTimeFactor } });

/** The room from the end of the words of a row to the start of the words of its *Change*. */
function roomAboveChange(row: Locator): Promise<number> {
  return row.evaluate((node) => {
    const [words, change] = [document.createRange(), document.createRange()];
    words.selectNodeContents(node.querySelector("[data-part='name-value.words']") ?? node);
    change.selectNodeContents([...node.querySelectorAll("button")].at(-1) ?? node);
    return change.getBoundingClientRect().top - words.getBoundingClientRect().bottom;
  });
}

/** The face an empty box writes its hint in, and the face of the text of the page. */
function facesOf(box: Locator): Promise<{ readonly hint: string; readonly text: string }> {
  return box.evaluate((node) => ({
    hint: getComputedStyle(node, "::placeholder").fontFamily,
    text: getComputedStyle(document.body).fontFamily,
  }));
}

test("ux_04_words_of_two_lines_keep_change_right_under_them", async ({ page, server }) => {
  await setFactorAs(server, ada, factorIssues.ada, 150);
  await signIn(page, bob);
  await openIssue(page, factorIssues.ada);
  await expect(factorRow(page)).toContainText(ada.name);
  // On a phone the words take two lines and *Change* the next: one gap between, no more.
  const room = await roomAboveChange(factorRow(page));
  expect(room).toBeLessThanOrEqual(20);
});

test("ux_04_the_hint_of_an_empty_box_for_a_length_is_in_the_face_of_the_text", async ({ page }) => {
  await signIn(page, ada);
  await openIssue(page, factorIssues.ada);
  const box = page.getByPlaceholder(t("fields.length-add")).first();
  await expect(box).toBeVisible();
  const faces = await facesOf(box);
  expect(faces.hint).toBe(faces.text);
});
