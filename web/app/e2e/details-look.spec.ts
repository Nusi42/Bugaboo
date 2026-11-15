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

// The look of rows of the Details (UX §4.4, R1), on a computer and a phone: words that take two
// lines keep *Change* a line under them with no hole between.

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

test("ux_04_words_of_two_lines_keep_change_right_under_them", async ({ page, server }) => {
  await setFactorAs(server, ada, factorIssues.ada, 150);
  await signIn(page, bob);
  await openIssue(page, factorIssues.ada);
  await expect(factorRow(page)).toContainText(ada.name);
  // On a phone the words take two lines and *Change* the next: one gap between, no more.
  const room = await roomAboveChange(factorRow(page));
  expect(room).toBeLessThanOrEqual(20);
});
