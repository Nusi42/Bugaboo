import { t } from "@bugaboo/messages";

import { ada, createAdmin } from "./support/accounts.ts";
import {
  chooseTheme,
  expectNoSidewaysScroll,
  expectNoViolations,
  themes,
  expectLook,
} from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn, signOut } from "./support/flows.ts";
import { paths } from "../src/shared/paths.ts";

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

for (const theme of themes) {
  test(`thm_01 the sign-in screen has no accessibility violations in ${theme.id}`, async ({
    page,
  }) => {
    await page.goto(paths.signIn);
    await expect(page.getByRole("heading", { level: 1, name: t("sign-in.title") })).toBeVisible();
    await chooseTheme(page, theme);
    await expectNoViolations(page);
  });
}

test("thm_01 the sign-in screen does not scroll sideways", async ({ page }) => {
  await page.goto(paths.signIn);
  await expectNoSidewaysScroll(page);
});

test("thm_02 Match my device follows the device and the choice is remembered", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await page.goto(paths.signIn);
  await expectLook(page, "dusk");
  await page.getByRole("radio", { name: t("theme.paper"), exact: true }).check();
  await page.reload();
  await expectLook(page, "paper");
  await page.getByRole("radio", { name: t("theme.device") }).check();
  await expectLook(page, "dusk");
});

test("thm_02 Match my device shows the resolved theme and follows the device live", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto(paths.signIn);
  const device = page.getByRole("radio", { name: t("theme.device") });
  await expect(device).toBeChecked();
  await expectLook(page, "daylight");
  await expect(device).toHaveAccessibleName(
    t("theme.device-resolved", { device: t("theme.device"), theme: t("theme.daylight") }),
  );
  await page.emulateMedia({ colorScheme: "dark" });
  await expectLook(page, "dusk");
  await expect(device).toHaveAccessibleName(
    t("theme.device-resolved", { device: t("theme.device"), theme: t("theme.dusk") }),
  );
});

test("thm_02 Match my device names the theme of the device even when another is chosen", async ({
  page,
}) => {
  await page.emulateMedia({ colorScheme: "light" });
  await page.goto(paths.signIn);
  await page.getByRole("radio", { name: t("theme.midnight"), exact: true }).check();
  await expectLook(page, "midnight");
  await expect(page.getByRole("radio", { name: t("theme.device") })).toHaveAccessibleName(
    t("theme.device-resolved", { device: t("theme.device"), theme: t("theme.daylight") }),
  );
});

test("thm_02 a theme chosen right after signing out stays while the page finishes the sign-out", async ({
  page,
}) => {
  await signIn(page, ada);
  // The question the sign-in page asks is still on its way for a while: the app keeps its cache
  // until it is answered, and the choice must not go to the person who has left meanwhile.
  await page.route("**/api/v1/auth/status", async (route) => {
    await new Promise<void>((resolve) => {
      setTimeout(resolve, 2500);
    });
    await route.continue();
  });
  await signOut(page);
  await page.getByRole("radio", { name: t("theme.midnight"), exact: true }).check();
  await expectLook(page, "midnight");
  // The wait is longer than the question: once it is answered the choice still stands.
  await page.waitForTimeout(4000);
  await expectLook(page, "midnight");
  await expect(page.getByRole("radio", { name: t("theme.midnight"), exact: true })).toBeChecked();
});
