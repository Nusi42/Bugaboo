import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { theTwo } from "./demo-m3-state.ts";
import { assign, good, openTheBug } from "./demo-m4-page.ts";
import { signIn } from "./flows.ts";
import { chooseStatus, expectStatus } from "./issues.ts";
import { serverWord } from "./server-words.ts";

// The steps of the exit demo of M4 that move the bug (`m4-exit-demo.spec.ts`): a Team member moves
// a bug of a Guest through the workflow of Operations, where the change that starts the work asks
// for who does it.

/** Step 1: the Guest's bug is open in Operations and nobody works on it yet. */
export async function theBugIsOpen(page: Page): Promise<void> {
  await signIn(page, theTwo().first);
  await openTheBug(page);
  await expectStatus(page, t("status.open"));
  await good(page, "01-the-bug-is-open");
}

/** Step 2: starting the work asks for who does it; the Team member names a colleague. */
export async function startingTheWorkAsksForWho(page: Page): Promise<void> {
  const { first: maker, second: colleague } = theTwo();
  await signIn(page, maker);
  await openTheBug(page);
  const word = serverWord("workflow-change.start-working");
  await chooseStatus(page, word);
  const window = page.getByRole("dialog", { name: word });
  await expect(window).toBeVisible();
  await assign(page, word, colleague);
  await expect(window.getByText(colleague.name)).toBeVisible();
  await good(page, "02-the-change-asks-for-who");
  await window.getByRole("button", { name: t("common.save"), exact: true }).click();
  await expectStatus(page, t("status.in-progress"));
  await expect(page.getByText(colleague.name).first()).toBeVisible();
  await good(page, "03-in-progress");
}

/** Step 4: the Team member marks the bug as done. */
export async function theBugIsDone(page: Page): Promise<void> {
  await signIn(page, theTwo().first);
  await openTheBug(page);
  await chooseStatus(page, serverWord("workflow-change.mark-as-done"));
  await expectStatus(page, t("status.done"));
  await good(page, "05-done");
}
