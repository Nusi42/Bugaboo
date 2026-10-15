import { t } from "@bugaboo/messages";
import { ada, bob } from "./support/accounts.ts";
import { signedInAdmin } from "./support/access.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { descriptionBox, openEditing, startEditing } from "./support/description.ts";
import { anotherBrowser, signIn } from "./support/flows.ts";
import { issueOf, prepareWeb, versionOf } from "./support/issues.ts";
import { holdBackLocks } from "./support/slow.ts";
import { issueAddress } from "../src/shared/paths.ts";

// Editing the description of an issue under its lock, and changing short values that somebody
// else changed first (ISS-17, ADR-0019, M3 WP-06): nobody loses text, and every refusal says what
// happened and what to do.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

test("iss_17 the description is saved, rendered by the server, and the lock is let go", async ({
  page,
}) => {
  await signIn(page, ada);
  await startEditing(page, "WEB-3");
  await descriptionBox(page).fill("It happens **every** Monday.");
  await page.getByRole("button", { name: t("common.save"), exact: true }).click();
  await expect(page.getByText(/^Saved/u).first()).toBeVisible();
  await expect(page.locator("strong", { hasText: "every" })).toBeVisible();
  await page.reload();
  await expect(page.locator("strong", { hasText: "every" })).toBeVisible();
});

test("iss_17 Ana types, Ben sees who is editing and uses Edit anyway, Ana's text is kept", async ({
  page,
  browser,
  server,
}) => {
  const ana = await anotherBrowser(browser, server);
  await signIn(ana, bob);
  await startEditing(ana, "WEB-2");
  await descriptionBox(ana).fill("Ana's text about the passkey");
  // Ben (an administrator) opens the same description and starts to type.
  await signIn(page, ada);
  // Ben is told who is editing when the box opens, before he types a key.
  await openEditing(page, "WEB-2");
  await expect(
    page.getByText(new RegExp(`^${bob.name} is editing the description`, "u")),
  ).toBeVisible();
  await expect(descriptionBox(page)).toHaveCount(0);
  await expectGoodInEveryLook(page);
  await page.getByRole("button", { name: t("editing.edit-anyway"), exact: true }).click();
  await descriptionBox(page).fill("Ben's text");
  await page.getByRole("button", { name: t("common.save"), exact: true }).click();
  await expect(page.getByText("Ben's text")).toBeVisible();
  // Ana saves: her time is over, her text is still in her box, and she can compare and save.
  await ana.getByRole("button", { name: t("common.save"), exact: true }).click();
  await expect(ana.getByText(/^Not saved yet/u).first()).toBeVisible();
  await expect(descriptionBox(ana)).toHaveValue("Ana's text about the passkey");
  await ana.getByRole("button", { name: t("issues.compare-and-save"), exact: true }).click();
  await expect(ana.getByText("Ben's text").first()).toBeVisible();
  await ana.getByRole("button", { name: t("issues.compare-save-mine"), exact: true }).click();
  await expect(ana.getByText("Ana's text about the passkey").first()).toBeVisible();
  await expect(ana.getByRole("button", { name: t("issues.description.edit") })).toBeVisible();
});

test("iss_17 Ben asks Ana to finish and Ana is told at her next heartbeat", async ({
  page,
  browser,
  server,
}) => {
  const ana = await anotherBrowser(browser, server);
  await signIn(ana, bob);
  await startEditing(ana, "WEB-3");
  await descriptionBox(ana).fill("Ana is writing");
  await signIn(page, ada);
  await openEditing(page, "WEB-3");
  await page
    .getByRole("button", { name: t("editing.ask-to-finish", { name: bob.name }), exact: true })
    .click();
  // Ana's page sends a heartbeat every 15 seconds and shows the note that comes back.
  await expect(ana.getByText(t("issues.asked-to-finish", { name: ada.name }))).toBeVisible({
    timeout: 30_000,
  });
});

test("iss_17 every key typed while the lock is still being asked for is kept", async ({ page }) => {
  await signIn(page, ada);
  await holdBackLocks(page, 1500);
  await page.goto(issueAddress("WEB-1"));
  await page.getByRole("button", { name: t("issues.description.edit"), exact: true }).click();
  // The first key replaces the text that is there, so the lock request is still on its way for the rest.
  await descriptionBox(page).press("ControlOrMeta+a");
  await descriptionBox(page).pressSequentially("Typed key by key", { delay: 120 });
  await expect(descriptionBox(page)).toHaveValue("Typed key by key");
  await page.getByRole("button", { name: t("common.save"), exact: true }).click();
  await expect(page.getByText(/^Saved/u).first()).toBeVisible();
  await page.reload();
  await expect(page.getByText("Typed key by key")).toBeVisible();
});

test("iss_17 Save pressed before the lock has arrived saves as soon as it does", async ({
  page,
}) => {
  await signIn(page, ada);
  await holdBackLocks(page, 1500);
  await page.goto(issueAddress("WEB-1"));
  await page.getByRole("button", { name: t("issues.description.edit"), exact: true }).click();
  await descriptionBox(page).fill("Typed in a hurry");
  await page.getByRole("button", { name: t("common.save"), exact: true }).click();
  await expect(page.getByText(/^Saved/u).first()).toBeVisible();
  await page.reload();
  await expect(page.getByText("Typed in a hurry")).toBeVisible();
});

test("adr_0019 a short value that somebody changed first shows Keep their value or Use mine", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const login = await issueOf(admin, "WEB-1");
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await expect(page.getByLabel(t("issue-field.priority"), { exact: true })).toBeVisible();
  // Somebody else changes the priority while the page is open.
  await admin.client.PATCH("/api/v1/issues/{issue}", {
    params: { path: { issue: login.key } },
    body: { version: await versionOf(admin, login), priority: "critical" },
  });
  await page.getByLabel(t("issue-field.priority"), { exact: true }).selectOption({
    label: t("priority.low"),
  });
  await expect(page.getByText(/changed Priority to Critical/u)).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.getByRole("button", { name: t("issues.use-mine"), exact: true }).click();
  await expect(page.getByText(/^Saved/u).first()).toBeVisible();
  await page.reload();
  await expect(page.getByLabel(t("issue-field.priority"), { exact: true })).toHaveValue("low");
});
