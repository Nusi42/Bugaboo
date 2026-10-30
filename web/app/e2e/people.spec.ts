import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, bob, cleo, createAdmin, createPerson } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { addressEndingAt, openFromMenu, signIn } from "./support/flows.ts";
import { itemArea, onThisLayout } from "./support/layout.ts";
import { paths } from "../src/shared/paths.ts";

// People and Groups (SEC-02, PRIV-12, WP-11): the list of people (a searchable list that draws only
// the rows on screen), the invitations that moved here, the page of a person, and the groups with
// the people in them, taking out and putting in with Undo.

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await (await createPerson(server, admin, bob)).dispose();
      await (await createPerson(server, admin, cleo)).dispose();
      await admin.dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** The list of people on the page *People*. */
function peopleList(page: Page) {
  return page.getByRole("listbox", { name: t("people.list"), exact: true });
}

test("sec_02 an administrator finds the people, searches them and opens one", async ({ page }) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.people"));
  await expect(page.getByRole("heading", { level: 1, name: t("nav.people") })).toBeVisible();
  const list = peopleList(page);
  // By name: Ada, Bob and Cleo, with their addresses.
  await expect(list.getByRole("option")).toHaveCount(3);
  await expect(list.getByRole("option", { name: /^Bob Builder/u })).toContainText(bob.email);
  await page.getByRole("searchbox", { name: t("people.search"), exact: true }).fill("coder");
  await expect(list.getByRole("option")).toHaveCount(1);
  await list.getByRole("option", { name: /^Cleo Coder/u }).click();
  await expect(page).toHaveURL(/\/settings\/people\/[^/]+$/u);
  await expect(page.getByRole("heading", { level: 1, name: cleo.name })).toBeVisible();
  await expect(page.getByText(cleo.email)).toBeVisible();
  // Her groups and roles: none yet.
  await expect(page.getByText(t("person.groups-none", { name: cleo.name }))).toBeVisible();
  await expect(page.getByText(t("person.roles-none", { name: cleo.name }))).toBeVisible();
  // On a phone the bar of the screen has Back, and the page does not say it twice.
  const back = page.getByRole("link", { name: t("person.back") });
  await onThisLayout(page, {
    computer: () => back.click(),
    tablet: () => back.click(),
    phone: async () => {
      await expect(back).toHaveCount(0);
      await page.goBack();
    },
  });
  await expect(page).toHaveURL(addressEndingAt(paths.people));
});

test("sec_02 the invitations are part of People and an invitation is made there", async ({
  page,
}) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.people"));
  await page.getByLabel(t("field.email"), { exact: true }).fill("newcomer@example.test");
  await page.getByRole("button", { name: t("invitations.create"), exact: true }).click();
  await expect(
    page.getByText(t("invitations.created", { email: "newcomer@example.test" })),
  ).toBeVisible();
  const open = page.getByRole("list", { name: t("invitations.open"), exact: true });
  await expect(open.getByText("newcomer@example.test")).toBeVisible();
});

test("sec_02 People, a person and Groups are good in every theme", async ({ page }) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.people"));
  await expect(peopleList(page).getByRole("option")).toHaveCount(3);
  await expectGoodInEveryLook(page);
  await peopleList(page)
    .getByRole("option", { name: /^Bob Builder/u })
    .click();
  await expect(page.getByRole("heading", { level: 1, name: bob.name })).toBeVisible();
  await expectGoodInEveryLook(page);
  await openFromMenu(page, t("nav.groups"));
  await expect(page.getByRole("heading", { level: 1, name: t("nav.groups") })).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("sec_02 a group is made, people are put into it and taken out with Undo", async ({ page }) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.groups"));
  await expect(page.getByText(t("groups.none-yet"))).toBeVisible();
  await page.getByLabel(t("groups.field-name"), { exact: true }).fill("Backend");
  await page.getByRole("button", { name: t("groups.create"), exact: true }).click();
  await expect(page.getByText(t("groups.created", { name: "Backend" }))).toBeVisible();
  await page.getByRole("button", { name: "Backend", exact: true }).click();
  const panel = itemArea(page, "Backend");
  await expect(panel).toBeVisible();
  await expect(panel.getByText(t("groups.members-none"))).toBeVisible();

  // Put Bob in: the picker finds him by name and says he is in afterwards.
  await panel.getByRole("searchbox", { name: t("groups.add-search") }).fill("builder");
  await panel.getByRole("button", { name: t("groups.add-name", { name: bob.name }) }).click();
  const members = panel.getByRole("list", { name: t("groups.members"), exact: true });
  await expect(members.getByText(bob.name)).toBeVisible();
  await expect(panel.getByText(t("groups.in-already"))).toBeVisible();
  await expect(
    page.getByText(t("groups.added", { name: bob.name, group: "Backend" })),
  ).toBeVisible();

  // Take him out at once, and take that back with Undo.
  await panel.getByRole("button", { name: t("groups.remove-name", { name: bob.name }) }).click();
  await expect(panel.getByText(t("groups.members-none"))).toBeVisible();
  // The offers wait one under the other: the latest one is the one that was just made.
  await page
    .getByRole("button", { name: t("common.undo"), exact: true })
    .last()
    .click();
  await expect(members.getByText(bob.name)).toBeVisible();
  await expectGoodInEveryLook(page);
});

test("sec_02 a group is deleted after a dialog that says what happens", async ({ page }) => {
  await signIn(page, ada);
  await page.goto(paths.groups);
  await page.getByRole("button", { name: "Backend", exact: true }).click();
  const panel = itemArea(page, "Backend");
  await panel.getByRole("button", { name: t("groups.delete-name", { name: "Backend" }) }).click();
  const dialog = page.getByRole("alertdialog");
  await expect(dialog.getByText(t("groups.delete-text", { members: 1 }))).toBeVisible();
  await dialog.getByRole("button", { name: t("groups.delete"), exact: true }).click();
  await expect(page.getByText(t("groups.none-yet"))).toBeVisible();
});
