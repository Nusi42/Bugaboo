import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { projectAddress, savedSearchAddress } from "../src/shared/paths.ts";
import { ada, createAdmin } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { createProject } from "./support/projects.ts";
import { openBar } from "./support/search-bar.ts";

// The accessibility suite for *Pinned* (CFG-05) and the window *E-mail me this search* (QRY-07) (TEST-A11Y): a pinned
// project and a pinned saved search in the sidebar (on a phone, first in *More*), and the window reached with the
// keyboard, in the four themes, on a computer and on a phone (the file is run for both).

const PROJECT = { shortName: "WEB", name: "Website" } as const;
const SEARCH = "Open bugs in Website";
let search = "";

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      const project = await createProject(admin, PROJECT.shortName, PROJECT.name);
      search = dataOf(
        await admin.client.POST("/api/v1/saved-searches", {
          body: { name: SEARCH, search_text: "" },
        }),
      ).id;
      await admin.client.POST("/api/v1/me/pins", {
        body: { kind: "project", target: project },
      });
      await admin.client.POST("/api/v1/me/pins", {
        body: { kind: "saved-search", target: search },
      });
      await admin.dispose();
    },
  },
});

test("a11y Pinned with a project and a saved search in the sidebar", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "a phone has Pinned in More, the next test");
  await signIn(page, ada);
  await page.goto(projectAddress(PROJECT.shortName));
  const pinned = page.getByRole("region", { name: t("pins.heading") });
  await expect(pinned.getByRole("link", { name: SEARCH, exact: true })).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y Pinned with a project and a saved search first in More on a phone", async ({ page }) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has the sheet More");
  await signIn(page, ada);
  await page.goto(projectAddress(PROJECT.shortName));
  await page.getByRole("button", { name: t("shell.more"), exact: true }).click();
  const sheet = page.getByRole("dialog", { name: t("shell.more") });
  await expect(sheet.getByRole("link", { name: SEARCH, exact: true })).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("a11y the window E-mail me this search, with the keyboard only", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(savedSearchAddress(search));
  await openBar(page);
  const button = page.getByRole("button", { name: t("search.mail-me"), exact: true });
  await button.focus();
  await page.keyboard.press("Enter");
  const window = page.getByRole("dialog", { name: t("search.mail-title", { name: SEARCH }) });
  await expect(window).toBeVisible();
  await expectGoodInEveryLook(page);
  await page.keyboard.press("Escape");
  await expect(window).toBeHidden();
});
