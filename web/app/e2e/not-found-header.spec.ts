import { t } from "@bugaboo/messages";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";
import { sampleTeamMember } from "./support/sample.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O (O10): a page that is hidden from a person is named "does not exist" in the top bar, not by the
// place under it (a hidden place must not be named).

test.use({ sample: true });

test("ux_3 the top bar of a hidden page does not name the place under it", async ({ page }) => {
  await signIn(page, sampleTeamMember());
  await page.goto(paths.people);
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeVisible();
  const bar = partIn(page, "top-bar");
  await expect(bar).toContainText(t("page.not-found.title"));
  await expect(bar).not.toContainText(t("nav.organization-settings"));
});
