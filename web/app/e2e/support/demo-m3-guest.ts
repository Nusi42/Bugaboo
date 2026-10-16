import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { issueAddress } from "../../src/shared/paths.ts";
import { goodInFourThemes } from "./demo.ts";
import { takeTheGuestsBug } from "./demo-m3-prepare.ts";
import { COST_CENTER, costCenter, GUEST_BUG, made, theTwo } from "./demo-m3-state.ts";
import { openIssueWithDetails, signIn } from "./flows.ts";
import { sampleGuestIn } from "./sample.ts";
import { type Instance } from "./server.ts";

// A step of the exit demo of M3 (`m3-exit-demo.spec.ts`): what a Guest sees of a bug that a Team
// member took.

/** Step 4: the Guest sees *Someone* for the person assigned to their bug, and no Cost center. */
export async function theGuestSeesSomeone(page: Page, server: Instance): Promise<void> {
  // What the Team member reads back shows that the change was made, so what the Guest does not
  // see below is hidden from them and not just missing.
  const taken = await takeTheGuestsBug(server);
  expect(taken.assigned).toEqual([theTwo().second.name]);
  expect(taken.cost).toBe(COST_CENTER);
  await signIn(page, sampleGuestIn("WEB"));
  // A Guest sees only the issues they made: the bug of the Team member is not there for them.
  await page.goto(issueAddress(made.key));
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeVisible();
  await openIssueWithDetails(page, made.guestKey);
  await expect(page.getByRole("heading", { name: GUEST_BUG })).toBeVisible();
  await expect(page.getByText(t("projects.person-someone"), { exact: true }).first()).toBeVisible();
  await Promise.all(
    taken.assigned.map((name) => expect(page.getByText(name, { exact: true })).toHaveCount(0)),
  );
  await expect(page.getByText(COST_CENTER)).toHaveCount(0);
  await expect(page.getByText(costCenter, { exact: true })).toHaveCount(0);
  await goodInFourThemes(page, "06-guest-sees-someone");
}
