import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { paths } from "../src/shared/paths.ts";
import { ada, bob, createAdmin, createPerson } from "./support/accounts.ts";
import { chooseTheme, themes, type Theme } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { screenshotFolder, shotWholeAs } from "./support/screenshots.ts";

// The kind of relation with its date effect (PLAN-02, ADR-0043), in the four themes, on a computer and
// on a phone, for the maintainer's review of M8 (WP-03). Only taken when `BB_SCREENSHOT_DIR` names
// the folder they go to:
//   BB_SCREENSHOT_DIR=... npx playwright test screenshots-relation-kinds.spec --project=desktop --project=phone
test.skip(screenshotFolder === undefined, "Set BB_SCREENSHOT_DIR to take the screenshots.");
test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await (await createPerson(server, admin, bob)).dispose();
      await admin.dispose();
    },
  },
});

const MILESTONE = "M8/relation-kinds";

/** The kind *blocks*, open, where its date effect can be chosen. */
async function openBlocks(page: Page): Promise<void> {
  await signIn(page, ada);
  await page.goto(paths.relationKinds);
  await page
    .getByRole("button", {
      name: t("relation-kinds.reads", { forward: "blocks", backward: "is blocked by" }),
      exact: true,
    })
    .click();
  await expect(page.getByLabel(t("relation-kinds.field-date-effect"), { exact: true })).toHaveValue(
    "must-finish-first",
  );
}

async function takeTheKind(page: Page, theme: Theme, project: string) {
  await openBlocks(page);
  await chooseTheme(page, theme);
  await shotWholeAs(page, `kind-with-date-effect-${project}`, MILESTONE, theme.id);
}

for (const theme of themes) {
  test(`the kind blocks with its date effect in ${theme.id}`, ({ page }, info) =>
    takeTheKind(page, theme, info.project.name));
}
