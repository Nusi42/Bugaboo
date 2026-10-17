import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { chooseTheme, themes } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareLook } from "./support/list-look.ts";
import { screenshotFolder, shotWholeAs } from "./support/screenshots.ts";

// The files block of the issue page and the files of a comment, in the four themes, for the
// maintainer's review of M4. Only taken when `BB_SCREENSHOT_DIR` names the folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-files.spec --project=desktop --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

const MILESTONE = "M4";
const MEGABYTE = 1024 * 1024;
const TOO_BIG_MEGABYTES = 26;

/** Adds files with *Upload* and waits until each one is a row. */
async function addFiles(page: Page, names: readonly string[]): Promise<void> {
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: t("files.upload"), exact: true }).click();
  await (
    await chooser
  ).setFiles(
    names.map((name) => ({
      name,
      mimeType: "application/octet-stream",
      buffer: Buffer.from(`the content of ${name}\n`),
    })),
  );
  await Promise.all(
    names.map((name) => expect(page.getByRole("listitem").filter({ hasText: name })).toBeVisible()),
  );
}

/** Chooses a file that is over the limit and waits for the refusal in words. */
async function refuseBigFile(page: Page): Promise<void> {
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: t("files.upload"), exact: true }).click();
  await (
    await chooser
  ).setFiles({
    name: "huge-video.bin",
    mimeType: "application/octet-stream",
    buffer: Buffer.alloc(TOO_BIG_MEGABYTES * MEGABYTE),
  });
  await expect(page.getByText(t("files.too-big", { limit: "25 MB" }))).toBeVisible();
}

test("files are added, and one that is too big is refused", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await addFiles(page, ["quarter-report.pdf", "crash.log", "screenshot.png"]);
  await refuseBigFile(page);
});

for (const theme of themes) {
  test(`files in ${theme.id}`, async ({ page }, info) => {
    await signIn(page, ada);
    await page.goto(issueAddress("WEB-1"));
    await expect(page.getByRole("listitem").filter({ hasText: "crash.log" })).toBeVisible();
    await chooseTheme(page, theme);
    await shotWholeAs(page, `files-${info.project.name}`, MILESTONE, theme.id);
  });
}
