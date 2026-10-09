import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, makeAdmin } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { paths } from "../src/shared/paths.ts";

test.use({
  preparation: {
    run: async (server) => {
      await (await makeAdmin(server)).dispose();
    },
  },
});

/** The text of the script that a person might paste into a description. */
const pasted = "<script>window.attacked = true</script>";

/** Opens the page that has the editor, and returns the text box. */
async function openEditor(page: Page) {
  await signIn(page, ada);
  await page.goto(paths.frames);
  const box = page.getByRole("textbox", { name: t("frames.text-label"), exact: true });
  await expect(box).toBeVisible();
  return box;
}

async function openPreview(page: Page): Promise<void> {
  await page.getByRole("tab", { name: t("markdown.preview"), exact: true }).click();
}

test("sec_13 a pasted script shows as text in the preview and never runs", async ({ page }) => {
  const box = await openEditor(page);
  await box.fill(`${pasted}\n\nSee **this**.`);
  await openPreview(page);
  const preview = page.getByRole("tabpanel", { name: t("markdown.preview"), exact: true });
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
  await page.getByRole("button", { name: t("markdown.bold"), exact: true }).click();
  await expect(box).toHaveValue("**one two**");
  await box.press("End");
  await page.keyboard.type(" and WEB-142 for @ana");
  await openPreview(page);
  const preview = page.getByRole("tabpanel", { name: t("markdown.preview"), exact: true });
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
  await expect(page.getByRole("heading", { level: 1, name: "Title" })).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("sec_13 Security shows the hosts that images in text may come from", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.organizationSecurity);
  await expect(
    page.getByRole("heading", { level: 1, name: t("nav.organization-security") }),
  ).toBeVisible();
  const hosts = page.getByRole("textbox", { name: t("setting.content.image-hosts.name") });
  await expect(hosts).toBeVisible();
  await expect(hosts).toHaveValue("");
  await expectGoodInEveryLook(page);
});
