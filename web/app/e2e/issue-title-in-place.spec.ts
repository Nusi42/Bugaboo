import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { partIn } from "./support/parts.ts";
import { countRequests } from "./support/requests.ts";
import { issuePageLandmarks } from "./support/landmarks.ts";
import { expectStill, movementWhile } from "./support/movement.ts";

// The title changed in place (ISS-03, UX §6.4): Enter saves it once, and the new title shows at once. The box goes away after Enter, which the
// browser also tells as a blur; a second save on the version before was refused as a conflict with
// the person's own change, and the page showed it.

test.use({ preparation: { run: prepareWeb } });

test("iss_03 Enter in the box of the title saves the title once, with no conflict", async ({
  page,
}) => {
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-1");
  await page.waitForLoadState("networkidle");
  const title = partIn(page, "issue-title");
  await title.getByRole("button").click();
  await title.getByRole("textbox").fill("Saved once");
  const saves = countRequests(page, "PATCH", "/api/v1/issues/");
  // The new title shows at once: never the old one for a frame, which on a phone is two lines high
  // and pushes the page down and up again.
  const around = await issuePageLandmarks(page, title);
  const saving = await movementWhile(page, around.all, async () => {
    await title.getByRole("textbox").press("Enter");
    await expect(page.getByRole("heading", { level: 1, name: "Saved once" })).toBeVisible();
  });
  expectStill(saving, around);
  expect(saves()).toBe(1);
});
