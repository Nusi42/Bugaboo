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

test("iss_03 the Enter that ends a composition of characters in the title saves nothing", async ({
  page,
}) => {
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-1");
  await page.waitForLoadState("networkidle");
  const title = partIn(page, "issue-title");
  await title.getByRole("button").click();
  const box = title.getByRole("textbox");
  await box.fill("Composed title");
  const saves = countRequests(page, "PATCH", "/api/v1/issues/");
  // An input method (IME) ends a composition with Enter; Safari says so only by the key code 229.
  // The box handles the key in the same turn: whether it took the key is known after it.
  const taken = (code: number) =>
    box.evaluate((element, keyCode) => {
      const press = new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true });
      Object.defineProperty(press, "keyCode", { value: keyCode });
      element.dispatchEvent(press);
      return press.defaultPrevented;
    }, code);
  expect(await taken(229)).toBe(false);
  await expect(box).toBeFocused();
  expect(saves()).toBe(0);
  expect(await taken(13)).toBe(true);
  await expect(page.getByRole("heading", { level: 1, name: "Composed title" })).toBeVisible();
  expect(saves()).toBe(1);
});
