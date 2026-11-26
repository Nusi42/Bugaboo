import { addressInProject, paths } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { changeButton, prepareReleases, releases } from "./support/releases.ts";

// The accessibility suite for the page *Releases* of a project (TEST-A11Y, UX §4.16, ISS-08): the tables,
// and the window of a release, reached with the keyboard only, in the four themes, on a computer and on a
// phone (the file is run for both).

test.use({ preparation: { run: prepareReleases } });

test("a11y the releases of a project and the window of a release, with the keyboard only", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(addressInProject(paths.projectReleases, "WEB"));
  const change = changeButton(page, releases.named);
  await expect(change).toBeVisible();
  await expectGoodInEveryLook(page);
  await change.focus();
  await page.keyboard.press("Enter");
  const window = page.getByRole("dialog");
  await expect(window).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await expect(window).toBeHidden();
});
