import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { chooseTheme, themes, type Theme } from "./support/checks.ts";
import {
  chooseFiles,
  holdSecondPart,
  limitInWords,
  limitOf,
  megabytes,
  overTheLimit,
  type Pictured,
  report,
  screenshot,
} from "./support/files.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareLook } from "./support/list-look.ts";
import { screenshotFolder, shotWholeAs } from "./support/screenshots.ts";

// The files block of the issue page and the files of a comment, in the four themes, for the
// maintainer's review of M4: finished previews, a refusal in words, a file under a comment, the bar of
// an upload on its way, and *Undo* after a file was deleted. Only taken when `BB_SCREENSHOT_DIR`
// names the folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-files.spec --project=desktop --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

const MILESTONE = "M4";
const ISSUE = "WEB-1";

/** How long the previews may take: the job runs every two seconds. */
const PREVIEW_WAIT_MS = 30_000;

function filesBlock(page: Page) {
  return page.getByRole("region", { name: t("files.title"), exact: true });
}

function rowOf(page: Page, name: string) {
  return page.getByRole("listitem").filter({ hasText: name });
}

/** Chooses files with *Upload* in the block of the issue. */
function chooseInBlock(page: Page, ...files: readonly Pictured[]): Promise<void> {
  return chooseFiles(
    page,
    filesBlock(page).getByRole("button", { name: t("files.upload"), exact: true }),
    files,
  );
}

/** A log of the printer, as a text file. */
const crashLog: Pictured = {
  name: "crash.log",
  mimeType: "text/plain",
  buffer: Buffer.from("The printer jams on page two."),
};

/** Writes a comment and chooses a picture for it, then posts it. */
async function commentWithPicture(page: Page): Promise<void> {
  const box = page.getByRole("textbox", { name: t("comments.write"), exact: true });
  await box.fill("The jam looks like this.");
  await chooseFiles(
    page,
    page.getByRole("button", { name: t("files.add-to-comment"), exact: true }),
    [screenshot("jam-on-page-two.png", 360, 220)],
  );
  await expect(
    page.getByRole("list", { name: t("files.for-comment") }).getByText("jam-on-page-two.png"),
  ).toBeVisible();
  await page
    .locator("form")
    .filter({ has: box })
    .getByRole("button", { name: t("comments.add"), exact: true })
    .click();
}

/** The state the pictures start from: a picture, a PDF and a log, and a comment with a picture. */
test("the issue gets its files and a comment with a file", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(issueAddress(ISSUE));
  await chooseInBlock(page, screenshot("screenshot.png", 480, 300), report(), crashLog);
  await commentWithPicture(page);
  // The previews are made by the job that runs every two seconds.
  await Promise.all(
    ["screenshot.png", "quarter-report.pdf", "jam-on-page-two.png"].map((name) =>
      expect(page.getByRole("img", { name: t("files.preview-of", { name }) })).toBeVisible({
        timeout: PREVIEW_WAIT_MS,
      }),
    ),
  );
});

/** The block with the finished previews, a file under a comment, and a refusal in words. */
async function takeTheBlock(page: Page, theme: Theme, project: string): Promise<void> {
  await signIn(page, ada);
  await page.goto(issueAddress(ISSUE));
  await expect(
    page.getByRole("img", { name: t("files.preview-of", { name: "screenshot.png" }) }),
  ).toBeVisible();
  // A refusal does not outlive the page, so it is made for this look.
  const limit = await limitOf(page, ISSUE);
  await chooseInBlock(page, overTheLimit("holiday-video.bin", limit));
  await expect(
    filesBlock(page).getByText(t("files.too-big", { limit: limitInWords(limit) })),
  ).toBeVisible();
  await chooseTheme(page, theme);
  await shotWholeAs(page, `files-${project}`, MILESTONE, theme.id);
}

/** An upload that stands still a part of the way, with its bar; the file is taken out after. */
async function takeTheBar(page: Page, theme: Theme, project: string): Promise<void> {
  await signIn(page, ada);
  await page.goto(issueAddress(ISSUE));
  const hold = await holdSecondPart(page);
  const name = `holiday-video-${theme.id}.bin`;
  await chooseInBlock(page, megabytes(name, 9));
  // The first part is there and the second waits: the bar is a part of the way.
  await expect(page.getByRole("progressbar")).toBeVisible();
  await chooseTheme(page, theme);
  await shotWholeAs(page, `files-progress-${project}`, MILESTONE, theme.id);
  hold.release();
  const remove = rowOf(page, name).getByRole("button", { name: t("files.delete-named", { name }) });
  await expect(remove).toBeVisible({ timeout: 30_000 });
  // The file is taken out again, so the next look starts from the same issue.
  await remove.click();
  await expect(rowOf(page, name).getByRole("button", { name: t("common.undo") })).toBeVisible();
}

/** A file that was deleted, with *Undo*; it is brought back after. */
async function takeUndo(page: Page, theme: Theme, project: string): Promise<void> {
  await signIn(page, ada);
  await page.goto(issueAddress(ISSUE));
  const name = "crash.log";
  await rowOf(page, name)
    .getByRole("button", { name: t("files.delete-named", { name }) })
    .click();
  await expect(rowOf(page, name).getByRole("button", { name: t("common.undo") })).toBeVisible();
  await chooseTheme(page, theme);
  await shotWholeAs(page, `files-undo-${project}`, MILESTONE, theme.id);
  await rowOf(page, name)
    .getByRole("button", { name: t("common.undo") })
    .click();
  await expect(
    rowOf(page, name).getByRole("button", { name: t("files.download-named", { name }) }),
  ).toBeVisible();
}

for (const theme of themes) {
  test.describe(`files in ${theme.id}`, () => {
    test("the block with previews, a file under a comment, and a refusal in words", ({
      page,
    }, info) => takeTheBlock(page, theme, info.project.name));
    test("an upload on its way, with its bar", ({ page }, info) =>
      takeTheBar(page, theme, info.project.name));
    test("a file that was deleted, with Undo", ({ page }, info) =>
      takeUndo(page, theme, info.project.name));
  });
}
