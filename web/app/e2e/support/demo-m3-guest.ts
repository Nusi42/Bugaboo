import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { issueAddress } from "../../src/shared/paths.ts";
import { goodInFourThemes } from "./demo.ts";
import { takeTheGuestsBug } from "./demo-m3-prepare.ts";
import { COST_CENTER, costCenter, GUEST_BUG, made } from "./demo-m3-state.ts";
import { signIn } from "./flows.ts";
import { sampleGuestIn } from "./sample.ts";
import { type Instance } from "./server.ts";

// A step of the exit demo of M3 (`m3-exit-demo.spec.ts`): what a Guest sees of a bug that a Team
// member took.

/** Step 4: the Guest sees *Someone* for the person assigned to their bug, and no Cost center. */
export async function theGuestSeesSomeone(page: Page, server: Instance): Promise<void> {
  await takeTheGuestsBug(server);
  await signIn(page, sampleGuestIn("WEB"));
  await page.goto(issueAddress(made.guestKey));
  await expect(page.getByRole("heading", { name: new RegExp(GUEST_BUG, "u") })).toBeVisible();
  await expect(page.getByText(t("projects.person-someone"), { exact: true }).first()).toBeVisible();
  await expect(page.getByText(COST_CENTER)).toHaveCount(0);
  await expect(page.getByText(costCenter, { exact: true })).toHaveCount(0);
  await goodInFourThemes(page, "06-guest-sees-someone");
}
