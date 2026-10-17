import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { untilTheLockIsLetGo } from "./support/description.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { issueAddress, paths } from "../src/shared/paths.ts";

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

/** The text of the script that a person might paste into a description. */
const pasted = "<script>window.attacked = true</script>";

/** Opens the description of an issue for editing, and returns the text box. */
/** The issue the tests of the editor write the description of. */
const ISSUE = "WEB-2";

async function openEditor(page: Page) {
  await signIn(page, ada);
  await page.goto(issueAddress(ISSUE));
  await page.getByRole("button", { name: t("issues.description.edit"), exact: true }).click();
  const box = page.getByRole("textbox", {
    name: t("issue-field.description"),
    exact: true,
  });
  await expect(box).toBeVisible();
  return box;
}

// Every test cancels its editing, so the lock is let go and the next test is not told that somebody
// is editing.
test.afterEach(async ({ page }) => {
  const cancel = page.getByRole("button", { name: t("common.cancel"), exact: true });
  if (await cancel.isVisible()) {
    await cancel.click();
    await expect(cancel).toBeHidden();
  }
  await untilTheLockIsLetGo(page, ISSUE);
});

async function openPreview(page: Page): Promise<void> {
  // The comment editor of the page has a Preview tab too: this is the one of the description.
  await page
    .getByRole("tablist", { name: t("issue-field.description"), exact: true })
    .getByRole("tab", { name: t("markdown.preview"), exact: true })
    .click();
}

test("sec_13 a pasted script shows as text in the preview and never runs", async ({ page }) => {
  const box = await openEditor(page);
  await box.fill(`${pasted}\n\nSee **this**.`);
  await openPreview(page);
  const preview = page.getByRole("tabpanel", {
    name: t("markdown.preview"),
    exact: true,
  });
  await expect(preview.getByText(pasted)).toBeVisible();
  await expect(preview.locator("strong")).toHaveText("this");
  expect(await page.evaluate(() => "attacked" in window)).toBe(false);
  await expect(preview.locator("script")).toHaveCount(0);
});

test("cf_01 the toolbar writes Markdown and the preview shows what others will see", async ({
  page,
}) => {
  const box = await openEditor(page);
  await box.fill("one two");
  await box.selectText();
  // The comment editor of the page has a toolbar too: this is the one around the description.
  await box
    .locator("xpath=ancestor::*[@data-part='markdown.editor'][1]")
    .getByRole("button", { name: t("markdown.bold"), exact: true })
    .click();
  await expect(box).toHaveValue("**one two**");
  await box.press("End");
  await page.keyboard.type(" and WEB-142 for @ana");
  await openPreview(page);
  const preview = page.getByRole("tabpanel", {
    name: t("markdown.preview"),
    exact: true,
  });
  await expect(preview.locator("strong")).toHaveText("one two");
  await expect(preview.locator("[data-kind='issue-id']")).toHaveText("WEB-142");
  await expect(preview.locator("[data-kind='mention']")).toHaveText("@ana");
});

test("iss_05 an empty text says there is nothing to preview", async ({ page }) => {
  await openEditor(page);
  await openPreview(page);
  await expect(page.getByText(t("markdown.preview-empty"))).toBeVisible();
});

test("ux_04 the editor is good in every theme, writing and previewing", async ({ page }) => {
  const box = await openEditor(page);
  await box.fill(
    "# Title\n\nSome *emphasis*, `code`, WEB-1 and @ana.\n\n> A quote\n\n- one\n- two",
  );
  await expectGoodInEveryLook(page);
  await openPreview(page);
  await expect(page.getByRole("heading", { level: 1, name: "Title", exact: true })).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("sec_13 Security shows the hosts that images in text may come from", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.organizationSecurity);
  await expect(
    page.getByRole("heading", {
      level: 1,
      name: t("nav.organization-security"),
    }),
  ).toBeVisible();
  const hosts = page.getByRole("textbox", {
    name: t("setting.content.image-hosts.name"),
  });
  await expect(hosts).toBeVisible();
  await expect(hosts).toHaveValue("");
  await expectGoodInEveryLook(page);
});

test("sec_13 a host list with an address in it is refused in plain words", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.organizationSecurity);
  const hosts = page.getByRole("textbox", {
    name: t("setting.content.image-hosts.name"),
  });
  await hosts.fill("https://images.example.com");
  await page.getByRole("button", { name: t("common.save"), exact: true }).click();
  await expect(page.getByText(t("error.setting.hosts-not-bare"))).toBeVisible();
});
