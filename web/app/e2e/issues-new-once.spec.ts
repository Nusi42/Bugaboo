import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import {
  newIssueDialog as dialog,
  openNewIssueDialog,
  prepareWeb,
  titleLabel,
} from "./support/issues.ts";
import { countRequests } from "./support/requests.ts";
import { holdUntilReleased } from "./support/slow.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// An issue is made once (ISS-11, M8 WP-13): after *Create the issue* the window reads the new issue
// before it opens it, and waits for that as it waited for the issue, so a second press makes nothing.

test.use({ preparation: { run: prepareWeb } });

/** The read of one issue by its readable ID: what the window does with the issue it created, before it opens it. */
const ISSUE_READ = /\/api\/v1\/issues\/WEB-\d+$/u;

/** Opens the window over the list of WEB with a title in it, and holds the read of the issue it creates. */
async function readyToCreate(page: Page, title: string) {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openNewIssueDialog(page);
  await dialog(page).getByLabel(titleLabel(), { exact: true }).fill(title);
  // The form can be sent: its type is read and chosen (WebKit can take seconds for that).
  await expect(
    dialog(page).getByRole("button", { name: t("issues.create"), exact: true }),
  ).toBeEnabled();
  // Whatever its readable ID, its read waits until it is let go.
  return holdUntilReleased(page, ISSUE_READ);
}

test("iss_11 a second press while the new issue is read before it opens makes no second issue", async ({
  page,
}) => {
  const held = await readyToCreate(page, "Only once");
  const made = countRequests(page, "POST", "/api/v1/projects/WEB/issues");
  const create = dialog(page).getByRole("button", { name: t("issues.create"), exact: true });
  await create.click();
  // The issue is made, and the window reads it before it opens it: now the second press comes.
  await held.asked;
  await create.dispatchEvent("click");
  held.release();
  await expect(page.getByRole("heading", { name: /Only once/u })).toBeVisible();
  expect(made()).toBe(1);
});

test("iss_11 Enter in the title while the new issue is read before it opens makes no second issue", async ({
  page,
}) => {
  const held = await readyToCreate(page, "Once with Enter");
  const made = countRequests(page, "POST", "/api/v1/projects/WEB/issues");
  const title = dialog(page).getByLabel(titleLabel(), { exact: true });
  await title.press("Enter");
  await held.asked;
  await title.press("Enter");
  held.release();
  await expect(page.getByRole("heading", { name: /Once with Enter/u })).toBeVisible();
  expect(made()).toBe(1);
});

test("iss_11 the Enter that ends a composition of characters sends nothing", async ({ page }) => {
  await readyToCreate(page, "Composed");
  const title = dialog(page).getByLabel(titleLabel(), { exact: true });
  // An input method (IME) ends a composition with Enter; Safari says so only by the key code 229.
  // A form is sent at once, in the same turn, when it is sent at all: the answer is known after it.
  const sent = (code: number) =>
    title.evaluate((box, keyCode) => {
      let submitted = false;
      const note = (): void => {
        submitted = true;
      };
      document.addEventListener("submit", note, { capture: true });
      const press = new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true });
      Object.defineProperty(press, "keyCode", { value: keyCode });
      box.dispatchEvent(press);
      document.removeEventListener("submit", note, { capture: true });
      return submitted;
    }, code);
  expect(await sent(229)).toBe(false);
  expect(await sent(13)).toBe(true);
});
