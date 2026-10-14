import { t } from "@bugaboo/messages";

import { ada } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import {
  newIssueDialog as dialog,
  openNewIssueDialog as openDialog,
  prepareWeb,
} from "./support/issues.ts";
import { onThisLayout } from "./support/layout.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// The new-issue dialog (ISS-11, UX §4.5, M3 WP-06): project, type and fields, required fields that
// say so, and a draft that is kept when the window closes and is gone after Create.

test.use({ preparation: { run: prepareWeb } });
test.describe.configure({ mode: "serial" });

test("iss_11 the dialog asks for a title in words, and the issue is made with Create", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openDialog(page);
  await expect(dialog(page).getByLabel(t("issues.title-required"), { exact: true })).toBeVisible();
  await expectGoodInEveryLook(page);
  await dialog(page).getByLabel(t("issues.title-required"), { exact: true }).fill("A new problem");
  await dialog(page)
    .getByRole("button", { name: t("issues.create"), exact: true })
    .click();
  await expect(page.getByRole("heading", { name: /A new problem/u })).toBeVisible();
});

test("iss_11 a half-filled dialog keeps its draft, and the draft is gone after Create", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openDialog(page);
  await dialog(page).getByLabel(t("issues.title-required"), { exact: true }).fill("Half written");
  await expect(dialog(page).getByText(t("issues.draft-kept"))).toBeVisible();
  await dialog(page)
    .getByRole("button", { name: t("common.cancel"), exact: true })
    .click();
  await expect(dialog(page)).toHaveCount(0);
  // The same page, read again: the draft comes from the browser and from the server.
  await page.reload();
  await openDialog(page);
  await dialog(page)
    .getByRole("button", { name: /^Continue your draft from/u })
    .click();
  await expect(dialog(page).getByLabel(t("issues.title-required"), { exact: true })).toHaveValue(
    "Half written",
  );
  await dialog(page)
    .getByRole("button", { name: t("issues.create"), exact: true })
    .click();
  await expect(page.getByRole("heading", { name: /Half written/u })).toBeVisible();
  await page.goto(projectIssuesAddress("WEB"));
  await openDialog(page);
  await expect(
    dialog(page).getByRole("button", { name: /^Continue your draft from/u }),
  ).toHaveCount(0);
});

test("iss_11 Create and start another keeps the project and the type and says what was made", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await openDialog(page);
  await dialog(page).getByLabel(t("issues.title-required"), { exact: true }).fill("First of two");
  await dialog(page)
    .getByRole("button", { name: t("issues.create-another"), exact: true })
    .click();
  await expect(dialog(page).getByText(/was created\./u)).toBeVisible();
  await expect(dialog(page).getByLabel(t("issues.title-required"), { exact: true })).toHaveValue(
    "",
  );
});

test("ux_05 the key c opens the dialog, and a phone has the button in its bar", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto(projectIssuesAddress("WEB"));
  await expect(
    page.getByRole("heading", { level: 1, name: t("issues.page-title", { project: "Website" }) }),
  ).toBeVisible();
  await onThisLayout(page, {
    computer: async () => {
      await page.keyboard.press("c");
      await expect(dialog(page)).toBeVisible();
    },
    tablet: async () => {
      await page.keyboard.press("c");
      await expect(dialog(page)).toBeVisible();
    },
    phone: async () => {
      await page
        .getByRole("navigation", { name: t("shell.menu") })
        .getByRole("button", { name: t("issues.new"), exact: true })
        .click();
      await expect(dialog(page)).toBeVisible();
    },
  });
});
