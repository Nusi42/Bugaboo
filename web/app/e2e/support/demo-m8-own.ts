import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { paths } from "../../src/shared/paths.ts";
import { signInThroughApi } from "./accounts.ts";
import {
  adaOfTheSample,
  colleagueOfTheSample,
  good,
  keyOf,
  leaOfTheSample,
  plan,
  titleOf,
} from "./demo-m8-common.ts";
import { signIn } from "./flows.ts";
import type { Instance } from "./server.ts";
import { factorRow, openIssue } from "./time-factor.ts";
import {
  READABLE,
  UNREADABLE,
  colorField,
  createFromDusk,
  hardColors,
  onlyThemeOf,
  readabilityLine,
  typeHint,
} from "./theme-editor.ts";

// Steps 4 to 6 of the exit demo of M8 (`m8-exit-demo.spec.ts`): Lea says an issue takes her ten days,
// the issue is given to someone else and its time factor goes back to ×1, and a theme for everyone
// that is hard to read is fixed and saved.

/** The work days the issue takes Lea, and the minutes in an hour. */
const TAKES_DAYS = 10;
const MINUTES_PER_HOUR = 60;
/** How long the issue takes Lea, as she types it: ten of her work days, in hours. */
const TAKES_ME = `${String((plan.usualMinutes * TAKES_DAYS) / MINUTES_PER_HOUR)}h`;
/** The name of the theme for everyone. */
const THEME_NAME = "Harbor";

/** The issue Lea tells the time of: the second of hers. */
async function leasIssue(server: Instance): Promise<string> {
  const session = await signInThroughApi(server, leaOfTheSample());
  const key = await keyOf(session, titleOf(1));
  await session.dispose();
  return key;
}

/** Step 4: as Lea, an issue takes ten days and shows as her own estimate. */
export async function leaSaysAnIssueTakesHerTenDays(page: Page, server: Instance): Promise<void> {
  const lea = leaOfTheSample();
  await signIn(page, lea);
  await openIssue(page, await leasIssue(server));
  await factorRow(page)
    .getByRole("button", { name: t("issues.time-factor-change-name") })
    .click();
  const window = page.getByRole("dialog", { name: t("issues.time-factor-ask-own") });
  await expect(window).toBeVisible();
  const length = window.getByLabel(t("issues.time-factor-length"), { exact: true });
  await length.fill(TAKES_ME);
  await length.press("Tab");
  await window.getByRole("button", { name: t("issues.time-factor-save") }).click();
  await expect(window).toBeHidden();
  await expect(factorRow(page)).toContainText(/own estimate/u);
  await good(page, "04-lea-says-ten-days");
}

/** Step 5: given to someone else, the time factor of the issue goes back to ×1. */
export async function givenToSomeoneElseTheFactorGoesBack(
  page: Page,
  server: Instance,
): Promise<void> {
  const [lea, colleague] = [leaOfTheSample(), colleagueOfTheSample()];
  await signIn(page, lea);
  await openIssue(page, await leasIssue(server));
  await expect(factorRow(page)).not.toContainText(t("issues.time-factor-as-estimated"));
  await page
    .getByRole("button", {
      name: t("issues.change-field", { field: t("issue-field.assigned-to") }),
    })
    .click();
  await page
    .getByRole("button", { name: t("issues.assign-name", { name: colleague.name }) })
    .click();
  await page.getByRole("button", { name: t("issues.unassign-name", { name: lea.name }) }).click();
  await page.getByRole("button", { name: t("common.done"), exact: true }).click();
  await expect(factorRow(page)).toContainText(t("issues.time-factor-as-estimated"));
  await good(page, "05-given-to-someone-else");
}

/** Step 6: a theme for everyone that is hard to read is refused until it is fixed, then saved. */
export async function anOwnThemeThatIsHardToReadIsFixed(
  page: Page,
  server: Instance,
): Promise<void> {
  const ada = adaOfTheSample();
  await signIn(page, ada);
  await page.goto(paths.themes);
  await createFromDusk(page, THEME_NAME);
  await typeHint(page, UNREADABLE);
  await expect(readabilityLine(page)).toContainText(
    t("theme-editor.readability-fix", { count: 1 }),
  );
  await page.getByRole("button", { name: t("theme-editor.save"), exact: true }).click();
  await expect(readabilityLine(page)).toHaveAttribute("data-state", "refused");
  await expect(hardColors(page)).toContainText(t("theme-color.fg-subtle"));
  expect((await onlyThemeOf(server, ada)).theme.saved).toBe(false);
  await good(page, "06-a-a-hard-to-read-theme-is-refused");
  await hardColors(page)
    .getByRole("button", {
      name: t("theme-editor.change-named", { name: t("theme-color.fg-subtle") }),
    })
    .click();
  await expect(colorField(page, t("theme-color.fg-subtle"))).toBeFocused();
  await typeHint(page, READABLE);
  await expect(readabilityLine(page)).not.toHaveAttribute("data-state");
  await page.getByRole("button", { name: t("theme-editor.save"), exact: true }).click();
  await expect.poll(async () => (await onlyThemeOf(server, ada)).theme.saved).toBe(true);
  await good(page, "06-b-the-theme-is-fixed-and-saved");
}
