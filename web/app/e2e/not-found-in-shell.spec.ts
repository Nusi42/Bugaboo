import { t } from "@bugaboo/messages";

import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";

// An issue that is not there (UX §6.1; M5 exit): said inside the shell, in its card, without the
// brand and the choice of theme of the pages outside it, and the top bar says what the card says.

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

test("ux_6_1 a page that is not found is the card inside the shell, not the sign-in layout", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto("/issues/NONE-1");
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeAttached();
  await expect(partIn(page, "shell.brand")).toHaveCount(0);
  await expect(page.getByRole("navigation", { name: t("shell.menu") })).toBeVisible();
  // The top bar says what the card says: on a phone its title, on a computer the last step of the trail.
  const top = page.getByRole("banner");
  await expect(top).toContainText(t("page.not-found.title"));
});
