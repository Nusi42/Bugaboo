import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { paths } from "../../src/shared/paths.ts";
import { supportText } from "../../src/features/about/index.ts";
import { downloadEveryFile } from "./about-words.ts";
import { type Instance } from "./accounts.ts";
import { adaThroughApi, good, lea } from "./demo-m9-common.ts";
import { signIn } from "./flows.ts";

// Step 7 of the exit demo of M9 (`m9-exit-demo.spec.ts`): *About* shows the version, the support
// period, the license and the parts list to download. That the release folder holds the same parts
// list, its signature and the notices is `cargo xtask release check` on the release (DEP-SBOM).

/** Step 7: the About page shows the release. */
export async function aboutTheRelease(page: Page, server: Instance): Promise<void> {
  const admin = await adaThroughApi(server);
  const about = dataOf(await admin.client.GET("/api/v1/about"));
  await admin.dispose();
  // Everybody who is signed in may open it: Lea, a Team member, does.
  await signIn(page, lea());
  await page.goto(paths.about);
  await expect(page.getByRole("heading", { level: 1, name: t("nav.about") })).toBeVisible();
  await expect(page.getByText(about.version, { exact: true })).toBeVisible();
  await expect(page.getByText(supportText(about.support_period), { exact: true })).toBeVisible();
  await expect(page.getByText(about.license, { exact: true })).toBeVisible();
  await good(page, "07-about-shows-the-release");
  await downloadEveryFile(page, about);
}
