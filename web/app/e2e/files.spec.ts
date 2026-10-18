import { partBytes } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import {
  breakSecondPartOnce,
  bytesOf,
  expectActionsOnTileEdge,
  dropFile,
  filesBlock,
  limitInWords,
  limitOf,
  megabytes,
  pasteFile,
  PREVIEW_WAIT_MS,
  previewOf,
  report,
  rowOf,
  screenshot,
  watchUploads,
} from "./support/files.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareLook } from "./support/list-look.ts";

// Files on an issue (M4 WP-05, ATT-01, ATT-03, ATT-04, UX §4.4): added by choosing, pasting and
// dropping, shown as rows with their preview, Download and Delete, refused in words when too big,
// resumed after an interruption, and good in every look.

test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

test("att_01 a file that is chosen with Upload is added, and is a row with Download and Delete", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await expect(filesBlock(page).getByText(t("files.none"))).toBeVisible();
  const chooser = page.waitForEvent("filechooser");
  await filesBlock(page)
    .getByRole("button", { name: t("files.upload"), exact: true })
    .click();
  await (
    await chooser
  ).setFiles({
    name: "report.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("the numbers of the quarter\n"),
  });
  const row = rowOf(page, "report.txt");
  await expect(row).toBeVisible();
  await expect(
    row.getByRole("button", { name: t("files.delete-named", { name: "report.txt" }) }),
  ).toBeVisible();
  await expect(
    row.getByRole("button", { name: t("files.download-named", { name: "report.txt" }) }),
  ).toBeVisible();
  // A file of a type that has no preview shows its type, and never a picture.
  await expect(row.getByRole("img")).toHaveCount(0);
  // The history has a row for the file, without a reload (M4-F50).
  await expect(
    page.getByText(t("history.file-added", { who: ada.name, name: "report.txt" })),
  ).toBeVisible();
  // Kept: the page is read again from the server.
  await page.reload();
  await expect(rowOf(page, "report.txt")).toBeVisible();
});

test("ux_4_4 a file's Download and Delete start on the edge of its tile, where the text of the row starts (R1)", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await expectActionsOnTileEdge(page, rowOf(page, "report.txt"), "report.txt");
});

test("att_04 a screenshot that is pasted is added and shows its preview as soon as it is made", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-2"));
  await expect(filesBlock(page)).toBeVisible();
  await pasteFile(page, screenshot("pasted.png"));
  await expect(rowOf(page, "pasted.png")).toBeVisible();
  // Nobody reloads the page: the list is read again until the picture is there.
  await expect(previewOf(page, "pasted.png")).toBeVisible({ timeout: PREVIEW_WAIT_MS });
});

test("att_04 a PDF that is dropped on the place is added and shows the first page", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-2"));
  await dropFile(
    filesBlock(page).getByRole("button", { name: t("files.drop-place"), exact: true }),
    report(),
  );
  await expect(rowOf(page, "quarter-report.pdf")).toBeVisible();
  await expect(previewOf(page, "quarter-report.pdf")).toBeVisible({ timeout: PREVIEW_WAIT_MS });
});

test("att_03 a file that is too big is refused in words that name the limit of the project", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const limit = await limitOf(page, "WEB-1");
  const chooser = page.waitForEvent("filechooser");
  await filesBlock(page)
    .getByRole("button", { name: t("files.upload"), exact: true })
    .click();
  await (await chooser).setFiles(bytesOf("huge.bin", limit + 1));
  await expect(
    filesBlock(page).getByText(t("files.too-big", { limit: limitInWords(limit) })),
  ).toBeVisible();
});

/** Chooses a file of nine megabytes, which goes in three parts, and waits until it is a row. */
async function sendBigFile(page: Page, name: string): Promise<void> {
  await page.goto(issueAddress("WEB-1"));
  const chooser = page.waitForEvent("filechooser");
  await filesBlock(page)
    .getByRole("button", { name: t("files.upload"), exact: true })
    .click();
  await (await chooser).setFiles(megabytes(name, 9));
  // The file is added when its row has *Download*: before that the row is the one that sends it.
  await expect(
    rowOf(page, name).getByRole("button", { name: t("files.download-named", { name }) }),
  ).toBeVisible({ timeout: 30_000 });
}

test.describe("att_01 an upload that is interrupted", () => {
  test("a part that did not arrive is sent again, and the parts before it are not", async ({
    page,
  }) => {
    await signIn(page, ada);
    const seen = watchUploads(page);
    await breakSecondPartOnce(page, false);
    await sendBigFile(page, "lost-part.bin");
    expect(seen.started).toBe(1);
    expect(seen.offsets).toEqual([0, partBytes, partBytes, 2 * partBytes]);
  });

  test("a part whose answer was lost is not sent again: the upload goes on from where the server stands", async ({
    page,
  }) => {
    await signIn(page, ada);
    const seen = watchUploads(page);
    await breakSecondPartOnce(page, true);
    await sendBigFile(page, "lost-answer.bin");
    expect(seen.started).toBe(1);
    expect(seen.offsets).toEqual([0, partBytes, 2 * partBytes]);
  });
});

test("att_01 a screenshot pasted into a comment box goes to that comment, not to the issue", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-3"));
  const box = page.getByRole("textbox", { name: t("comments.write"), exact: true });
  await box.fill("Look at this.");
  await pasteFile(page, screenshot("in-the-comment.png"));
  // Chosen for the comment, and not sent anywhere yet.
  const chosen = page.getByRole("list", { name: t("files.for-comment") });
  await expect(chosen.getByText("in-the-comment.png")).toBeVisible();
  await expect(rowOf(page, "in-the-comment.png")).toHaveCount(0);
  await page
    .locator("form")
    .filter({ has: box })
    .getByRole("button", { name: t("comments.add"), exact: true })
    .click();
  const files = page.getByRole("list", { name: t("files.of-comment") });
  await expect(files.getByText("in-the-comment.png")).toBeVisible();
  // Under the comment, not in the block of the issue.
  await expect(filesBlock(page).getByText("in-the-comment.png")).toHaveCount(0);
});

test("att_01 a file is deleted and comes back with Undo", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const row = rowOf(page, "report.txt");
  await row.getByRole("button", { name: t("files.delete-named", { name: "report.txt" }) }).click();
  await expect(
    rowOf(page, "report.txt").getByRole("button", { name: t("common.undo") }),
  ).toBeVisible();
  await rowOf(page, "report.txt")
    .getByRole("button", { name: t("common.undo") })
    .click();
  await expect(
    rowOf(page, "report.txt").getByRole("button", {
      name: t("files.download-named", { name: "report.txt" }),
    }),
  ).toBeVisible();
});

test("ux_4_4 the files block is good in every look", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await expect(rowOf(page, "report.txt")).toBeVisible();
  await expectGoodInEveryLook(page);
});
