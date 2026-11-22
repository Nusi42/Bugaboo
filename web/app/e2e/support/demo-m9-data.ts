import { readFileSync } from "node:fs";

import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { paths } from "../../src/shared/paths.ts";
import { ada, ben, good, lea } from "./demo-m9-common.ts";
import { signIn, signOut } from "./flows.ts";
import { type Mailbox } from "./mailbox.ts";
import { serverWord } from "./server-words.ts";
import { textIn } from "./zip.ts";

// Step 5 of the exit demo of M9 (`m9-exit-demo.spec.ts`): Lea gets her file from *My data*, with the
// purposes, legal bases, keep periods and recipients in its README; Ada makes Ben's in *Data
// exports* with a reason, and Ben is told by e-mail.

/** The README of the file of a person's data. */
const README = "README.txt";

/** Why Ada asks for Ben's data. */
const REASON = "Ben asked for his data by e-mail";

/** Lea downloads the file of her data, which is ready in the sample, and reads its README. */
async function leaDownloadsHerData(page: Page): Promise<void> {
  await signIn(page, lea());
  await page.goto(paths.myData);
  const download = page.getByRole("region", { name: t("my-data.download"), exact: true });
  const file = download.getByRole("button", { name: t("my-data.download-file"), exact: true });
  await expect(file).toBeVisible();
  await good(page, "05-lea-downloads-her-data");
  const saved = page.waitForEvent("download");
  await file.click();
  const path = await (await saved).path();
  const readme = textIn(readFileSync(path), README);
  for (const heading of [
    serverWord("export.readme.purpose"),
    serverWord("export.readme.basis"),
    serverWord("export.readme.kept"),
    serverWord("export.readme.recipients"),
  ]) {
    expect(readme).toContain(heading);
  }
  await signOut(page);
}

/** Ada asks for Ben's data with a reason in *Data exports*; Ben is told by e-mail. */
async function adaDownloadsBensData(page: Page, mailbox: Mailbox): Promise<void> {
  await signIn(page, ada());
  await page.goto(paths.dataExports);
  await page.getByRole("button", { name: t("data-exports.someone"), exact: true }).click();
  const window = page.getByRole("dialog", { name: t("data-exports.someone") });
  await window.getByRole("searchbox", { name: t("data-exports.pick-search") }).fill("Ben");
  await window.getByRole("radio", { name: ben().name }).check();
  await window.getByRole("textbox", { name: t("data-exports.reason") }).fill(REASON);
  // Downloading someone's data asks for the password again.
  await window.getByLabel(t("field.confirm-password"), { exact: true }).fill(ada().password);
  await good(page, "05-ada-asks-for-bens-data");
  await window.getByRole("button", { name: t("data-exports.create"), exact: true }).click();
  await expect(window).toBeHidden();
  const row = page
    .getByRole("table", { name: t("data-exports.list") })
    .getByRole("row")
    .filter({ hasText: REASON });
  await expect(row.getByRole("rowheader")).toHaveText(ben().name);
  await expect(row).toContainText(ada().name);
  await good(page, "05-bens-export-in-data-exports");
  // Ben is told, with the reason, though he is switched off.
  const mail = await mailbox.waitFor(ben().email);
  expect(mail.text).toContain(REASON);
}

/** Step 5: Lea downloads her data, and an administrator downloads Ben's. */
export async function leaAndAnAdministratorDownloadData(
  page: Page,
  mailbox: Mailbox,
): Promise<void> {
  await leaDownloadsHerData(page);
  await adaDownloadsBensData(page, mailbox);
}
