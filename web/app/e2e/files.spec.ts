import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareLook } from "./support/list-look.ts";

// Files on an issue (M4 WP-05, ATT-01, ATT-03, UX §4.4): added by choosing, pasting and dropping,
// shown as rows with Download and Delete, refused in words when too big, and good in every look.

test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

const MEGABYTE = 1024 * 1024;
/** The default limit of one file (the setting `files.max-megabytes`). */
const LIMIT_MEGABYTES = 25;

/** A picture of one pixel, as a screenshot from the clipboard would come. */
const ONE_PIXEL_PNG =
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/q842iQAAAABJRU5ErkJggg==";

function filesBlock(page: Page) {
  return page.getByRole("region", { name: t("files.title"), exact: true });
}

/** The row of a file by its name. */
function rowOf(page: Page, name: string) {
  return filesBlock(page).getByRole("listitem").filter({ hasText: name });
}

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
  // Kept: the page is read again from the server.
  await page.reload();
  await expect(rowOf(page, "report.txt")).toBeVisible();
});

test("att_01 a screenshot that is pasted is added as a picture", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-2"));
  await expect(filesBlock(page)).toBeVisible();
  await page.evaluate((base64) => {
    const bytes = Uint8Array.from(atob(base64), (character) => Number(character.codePointAt(0)));
    const data = new DataTransfer();
    data.items.add(new File([bytes], "screenshot.png", { type: "image/png" }));
    document.dispatchEvent(new ClipboardEvent("paste", { clipboardData: data, bubbles: true }));
  }, ONE_PIXEL_PNG);
  await expect(rowOf(page, "screenshot.png")).toBeVisible();
});

test("att_03 a file that is too big is refused in words that name the limit", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const chooser = page.waitForEvent("filechooser");
  await filesBlock(page)
    .getByRole("button", { name: t("files.upload"), exact: true })
    .click();
  await (
    await chooser
  ).setFiles({
    name: "huge.bin",
    mimeType: "application/octet-stream",
    buffer: Buffer.alloc((LIMIT_MEGABYTES + 1) * MEGABYTE),
  });
  await expect(filesBlock(page).getByText(t("files.too-big", { limit: "25 MB" }))).toBeVisible();
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
