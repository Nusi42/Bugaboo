import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { filled, issueAddress, paths, routeParam } from "../../src/shared/paths.ts";
import { type Instance } from "./accounts.ts";
import { keyOf, titleOf } from "./demo-m8-common.ts";
import { ada, adaThroughApi, ben, good, hana } from "./demo-m9-common.ts";
import { formerUser } from "./former-user.ts";
import { signIn, signOut } from "./flows.ts";
import { dayFromToday } from "./person-day.ts";
import { idInPeople, openPersonById, removalWindow, stateCard } from "./privacy.ts";

// Step 6 of the exit demo of M9 (`m9-exit-demo.spec.ts`): the window *Remove a person* says what
// goes, what stays and that Ben's work days and his name for them are kept until their date; after it
// Ben is *Former user* on his issues and comments, and Hana (*HR*) still sees his work days under his
// name.

/** The days of a week. */
const WEEK = 7;

/** The issue of the plan of the sample that Ben works on (*Push notifications*). */
const BENS_ISSUE = 3;

/** Ada removes Ben, who is switched off since step 4, from his page. */
async function adaRemovesBen(page: Page, id: string): Promise<void> {
  const name = ben().name;
  await signIn(page, ada());
  await openPersonById(page, id, name);
  await stateCard(page)
    .getByRole("button", { name: t("person.remove") })
    .click();
  const window = removalWindow(page, name);
  await Promise.all(
    [t("removal.deleted-now"), t("removal.stays"), t("removal.kept-by-law")].map((heading) =>
      expect(window.getByRole("heading", { name: heading, exact: true })).toBeVisible(),
    ),
  );
  // His work days are kept by law, and his name with them, until a day the window names.
  await expect(window.getByRole("list", { name: t("removal.kept-by-law") })).toContainText(
    t("keep-kind.name.work-days"),
  );
  // Removing asks for the password again.
  await window.getByLabel(t("field.confirm-password"), { exact: true }).fill(ada().password);
  await good(page, "06-the-window-says-what-goes-and-what-stays");
  await window.getByRole("button", { name: t("removal.confirm", { name }) }).click();
  await expect(window).toBeHidden();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(formerUser);
  await good(page, "06-ben-is-former-user");
}

/** Step 6: Ben is removed. */
export async function benIsRemoved(page: Page, server: Instance): Promise<void> {
  const admin = await adaThroughApi(server);
  const id = await idInPeople(admin, ben());
  const issue = await keyOf(admin, titleOf(BENS_ISSUE));
  await admin.dispose();
  await adaRemovesBen(page, id);
  // On his issue he is Former user, and his name is gone.
  await page.goto(issueAddress(issue));
  await expect(page.getByText(formerUser).first()).toBeVisible();
  await expect(page.getByText(ben().name)).toHaveCount(0);
  await good(page, "06-bens-issue-names-former-user");
  // Ada signs out from the home page, whose menu has *Sign out* on a phone too.
  await page.goto(paths.home);
  await signOut(page);
  // Hana, who holds HR, still sees his work days under his name.
  await signIn(page, hana());
  // The week before this one, which the sample filled with his days.
  await page.goto(
    `${filled(paths.timeOfPerson, routeParam.person, id)}?week=${dayFromToday(-WEEK)}`,
  );
  await expect(page.getByRole("heading", { level: 1, name: t("time.others.title") })).toBeVisible();
  await expect(page.getByText(ben().name).first()).toBeVisible();
  await expect(page.getByText(t("time.sheet.empty"))).toHaveCount(0);
  await good(page, "06-hana-sees-bens-work-days");
}
