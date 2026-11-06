import { t } from "@bugaboo/messages";
import { type Locator } from "@playwright/test";

import { issueAddress, paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { issueMoreOf } from "./support/layout.ts";
import { longScene } from "./support/time-scene.ts";

// The warning of a timer that runs longer than a work day floats over the page at the bottom of the
// start side (UX §4.9): it steps aside while a field is typed in, and the offers to undo stand
// beside it. A spec for a computer.

test.use({ preparation: { run: prepareWeb } });
test.setTimeout(180_000);

/** Whether the boxes of two things on the page overlap. */
async function overlap(first: Locator, second: Locator): Promise<boolean> {
  const [one, other] = await Promise.all([first.boundingBox(), second.boundingBox()]);
  if (one === null || other === null) {
    return false;
  }
  return (
    one.x < other.x + other.width &&
    other.x < one.x + one.width &&
    one.y < other.y + other.height &&
    other.y < one.y + one.height
  );
}

test("time_01 the warning of a long timer steps aside while a field is typed in", async ({
  page,
  server,
}) => {
  const { issue, dispose } = await longScene(server);
  await signIn(page, ada);
  await page.goto(paths.time);
  const note = page
    .getByRole("status")
    .filter({ hasText: t("time.timer.runs-long-on", { issue: issue.key }) });
  await expect(note).toBeVisible();
  const field = page.getByRole("textbox").first();
  await field.focus();
  await expect(note).toBeHidden();
  await field.blur();
  await expect(note).toBeVisible();
  await dispose();
});

test("time_01 the offers to undo stand beside the warning of a long timer, not over it", async ({
  page,
  server,
}) => {
  const { issue, dispose } = await longScene(server);
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-2"));
  const note = page
    .getByRole("status")
    .filter({ hasText: t("time.timer.runs-long-on", { issue: issue.key }) });
  await expect(note).toBeVisible();
  await issueMoreOf(page).click();
  await page.getByRole("button", { name: t("issues.delete"), exact: true }).click();
  const offer = page.getByText(t("issues.deleted", { key: "WEB-2" }));
  await expect(offer).toBeVisible();
  await expect(note).toBeVisible();
  expect(await overlap(note, offer)).toBe(false);
  await dispose();
});
