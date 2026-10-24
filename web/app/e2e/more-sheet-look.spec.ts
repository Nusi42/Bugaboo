import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { paths } from "../src/shared/paths.ts";
import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { nothing, onThisLayout } from "./support/layout.ts";
import { textStartOf } from "./support/windows.ts";

// The *More* sheet of a phone (the final check of M4): its heading *Projects* starts where
// *Signed in as* starts.

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

async function headingsStartTogether(page: Page): Promise<void> {
  await page.goto(paths.home);
  await page.getByRole("button", { name: t("shell.more"), exact: true }).click();
  const sheet = page.getByRole("dialog", { name: t("shell.more") });
  const who = await textStartOf(sheet.getByText(t("shell.signed-in-as", { name: ada.name })));
  const projects = await textStartOf(sheet.getByRole("heading", { name: t("nav.projects") }));
  expect(projects).toBe(who);
}

test("ux_01 in the More sheet the heading Projects starts where Signed in as starts", async ({
  page,
}) => {
  await signIn(page, ada);
  await onThisLayout(page, {
    phone: headingsStartTogether,
    tablet: nothing,
    computer: nothing,
  });
});
