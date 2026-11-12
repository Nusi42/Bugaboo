import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";

import { roleId, roleName, setFieldLevel } from "./support/access.ts";
import { ada, bob, signInThroughApi } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn, signOut } from "./support/flows.ts";
import {
  factorAs,
  factorIssues,
  factorRow,
  historyLine,
  lea,
  openIssue,
  prepareTimeFactor,
  setFactorAs,
} from "./support/time-factor.ts";

// The time factor on the issue page (PLAN-14, ADR-0030 decision 4 and amendment M8, UX §4.4): the
// person on the bar gives it as a length, it goes back to ×1 when the first assigned person changes,
// and a role whose field level hides it does not see it, while the person on the bar always does.

test.use({ preparation: { run: prepareTimeFactor } });

test("plan_14_the_person_on_the_bar_sets_the_time_factor_with_takes_me", async ({
  page,
  server,
}) => {
  await signIn(page, bob);
  await openIssue(page, factorIssues.ada);
  await expect(factorRow(page)).toContainText(t("issues.time-factor-as-estimated"));
  await factorRow(page)
    .getByRole("button", { name: t("issues.time-factor-change-name") })
    .click();
  const window = page.getByRole("dialog", { name: t("issues.time-factor-ask-own") });
  await expect(window).toBeVisible();
  await expect(window.getByRole("radio", { name: t("issues.time-factor-takes-me") })).toBeChecked();
  const length = window.getByLabel(t("issues.time-factor-length"), { exact: true });
  await length.fill("10h");
  await length.press("Tab");
  // The sum says the time factor the server will work out, and what the plan counts with it.
  await expect(window.getByText(/8h · ×1[.,]25 = 10h/u)).toBeVisible();
  await window.getByRole("button", { name: t("issues.time-factor-save") }).click();
  await expect(window).toBeHidden();
  await expect.poll(async () => (await factorAs(server, bob, factorIssues.ada)).percent).toBe(125);
  await expect(factorRow(page)).toContainText(/×1[.,]25/u);
  await expect(factorRow(page)).toContainText(bob.name);
  // The history writes the time factor as the row does, never as its percent.
  await expect(historyLine(page, String.raw`×1[.,]25`).first()).toBeVisible();
  await expect(historyLine(page, "125")).toHaveCount(0);
});

test("plan_14_someone_else_sets_the_time_factor_and_the_issue_says_who", async ({ page }) => {
  await signIn(page, ada);
  await openIssue(page, factorIssues.ada);
  await factorRow(page)
    .getByRole("button", { name: t("issues.time-factor-change-name") })
    .click();
  const window = page.getByRole("dialog", {
    name: t("issues.time-factor-ask-other", { name: bob.name }),
  });
  await expect(window.getByText(t("issues.time-factor-told", { name: bob.name }))).toBeVisible();
  await window.getByRole("radio", { name: t("issues.time-factor-as-factor") }).click();
  const factor = window.getByLabel(t("issues.time-factor-factor"), { exact: true });
  await factor.fill("1.5");
  await expect(window.getByText(/8h · ×1[.,]5 = 12h/u)).toBeVisible();
  await window.getByRole("button", { name: t("issues.time-factor-save") }).click();
  await expect(window).toBeHidden();
  await expect(factorRow(page)).toContainText(/×1[.,]5 · set by Ada Admin/u);
});

test("plan_14_the_time_factor_goes_back_to_x1_on_screen_when_the_person_changes", async ({
  page,
  server,
}) => {
  await setFactorAs(server, ada, factorIssues.ada, 150);
  await signIn(page, ada);
  await openIssue(page, factorIssues.ada);
  await expect(factorRow(page)).toContainText(/×1[.,]5 .*Ada Admin/u);
  // Ada puts herself in, then takes Bob out: she is the first assigned person now.
  const assigned = page.getByRole("button", {
    name: t("issues.change-field", { field: t("issue-field.assigned-to") }),
  });
  await assigned.click();
  await page.getByRole("button", { name: t("issues.assign-name", { name: ada.name }) }).click();
  await page.getByRole("button", { name: t("issues.unassign-name", { name: bob.name }) }).click();
  await page.getByRole("button", { name: t("common.done"), exact: true }).click();
  await expect(factorRow(page)).toContainText(t("issues.time-factor-as-estimated"));
  expect((await factorAs(server, ada, factorIssues.ada)).percent).toBe(100);
});

test("plan_14_a_field_level_that_hides_the_time_factor_hides_its_row", async ({ page, server }) => {
  await setFactorAs(server, ada, factorIssues.lea, 150);
  // Lea is a Guest, whose role does not see the time factor at the start.
  await signIn(page, lea);
  const answered = page.waitForResponse((answer) => answer.url().endsWith("/time-factor"));
  await openIssue(page, factorIssues.lea);
  await answered;
  await expect(page.getByText(t("issue-field.estimated-time"), { exact: true })).toHaveCount(0);
  await expect(factorRow(page)).toHaveCount(0);
  // Hidden from Team members too: Bob is the person on the bar and still sees his own factor.
  const admin = await signInThroughApi(server, ada);
  await setFieldLevel(
    admin,
    { role: await roleId(admin, roleName.teamMember), field: "time-factor" },
    "hidden",
  );
  await admin.dispose();
  // The sign-out is in the menu of the start page; the Details of a phone stand over the page.
  await page.goto(paths.home);
  await signOut(page);
  await signIn(page, bob);
  await openIssue(page, factorIssues.lea);
  await expect(factorRow(page)).toContainText(/×1[.,]5/u);
});
