import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { type ApiSession } from "./support/api.ts";
import { ada, bob, cleo, invite, createAdmin, createPerson } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import {
  anotherBrowser,
  expectHome,
  historyAddresses,
  openFromMenu,
  openMenu,
  signIn,
} from "./support/flows.ts";
import { inOrder } from "./support/sequence.ts";
import { paths } from "../src/shared/paths.ts";

/** The administrator, signed in through the API, for making many invitations quickly. */
let admin: ApiSession;

test.use({
  preparation: {
    run: async (server) => {
      admin = await createAdmin(server);
      await (await createPerson(server, admin, cleo)).dispose();
    },
  },
});
test.describe.configure({ mode: "serial" });

/** The page of invitations, opened by an administrator. */
async function openInvitations(page: Page): Promise<void> {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.people"));
  await expect(page.getByRole("heading", { level: 1, name: t("nav.people") })).toBeVisible();
}

/** Makes an invitation for `email` on the page, and returns the link that is shown. */
async function createInvitation(page: Page, email: string): Promise<string> {
  await page.getByLabel(t("field.email"), { exact: true }).fill(email);
  await page.getByRole("button", { name: t("invitations.create") }).click();
  await expect(page.getByText(t("invitations.created", { email }))).toBeVisible();
  const link = (await page.getByLabel(t("invitations.link")).textContent()) ?? "";
  expect(link).toMatch(/\/invitation#\S+/u);
  return link;
}

/** Fills in the name and password of an invitation and sends them. */
async function acceptInvitation(page: Page, name: string, password: string): Promise<void> {
  await page.getByLabel(t("field.name")).fill(name);
  await page.getByLabel(t("field.password")).fill(password);
  await page.getByRole("button", { name: t("invitation.submit") }).click();
}

test("auth_06 an administrator makes an invitation and the person makes an account with its link", async ({
  page,
  browser,
  server,
}) => {
  await openInvitations(page);
  const link = await createInvitation(page, bob.email);
  await page.getByRole("button", { name: t("invitations.copy") }).click();
  await expect(page.getByRole("button", { name: t("invitations.copied") })).toBeVisible();
  await expect(
    page.getByRole("list", { name: t("invitations.open") }).getByText(bob.email),
  ).toBeVisible();

  const guest = await anotherBrowser(browser, server);
  await guest.goto(link);
  await expect(guest.getByRole("heading", { level: 1, name: t("invitation.title") })).toBeVisible();
  await expect(guest.getByText(t("invitation.intro", { email: bob.email }))).toBeVisible();
  await acceptInvitation(guest, bob.name, bob.password);
  await expectHome(guest, bob);
  // The secret is used up and leaves the address and the history of the browser.
  const secret = link.slice(link.indexOf("#") + 1);
  expect(await historyAddresses(guest)).not.toContain(secret);
  await guest.context().close();
});

test("auth_06 the link of an invitation works once", async ({ page, browser, server }) => {
  const link = await invite(admin, "frank@example.test");
  const first = await anotherBrowser(browser, server);
  await first.goto(link);
  await acceptInvitation(first, "Frank", "Maple-Harvest-Quartz-2291");
  await expect(
    first.getByRole("heading", { level: 1, name: t("home.title", { name: "Frank" }) }),
  ).toBeVisible();

  await page.goto(link);
  await expect(
    page.getByRole("heading", { level: 1, name: t("invitation.invalid-title") }),
  ).toBeVisible();
  await expect(page.getByRole("alert")).toHaveText(t("error.auth.invitation-invalid"));
  await first.context().close();
});

test("auth_01 a weak password is explained and the invitation stays usable", async ({ page }) => {
  await page.goto(await invite(admin, "dora@example.test"));
  await acceptInvitation(page, "Dora", "short");
  await expect(page.getByText(t("error.auth.password-too-short"))).toBeVisible();
  await acceptInvitation(page, "Dora", "Maple-Harvest-Quartz-2291");
  await expect(
    page.getByRole("heading", { level: 1, name: t("home.title", { name: "Dora" }) }),
  ).toBeVisible();
});

test("auth_06 an invitation that is taken back stops working", async ({
  page,
  browser,
  server,
}) => {
  const email = "erin@example.test";
  const link = await invite(admin, email);
  await openInvitations(page);
  const open = page.getByRole("list", { name: t("invitations.open") });
  await expect(open.getByText(email)).toBeVisible();
  await page.getByRole("button", { name: t("invitations.take-back-name", { email }) }).click();
  await expect(page.getByRole("alertdialog")).toContainText(
    t("invitations.take-back-title", { email }),
  );
  await page
    .getByRole("alertdialog")
    .getByRole("button", { name: t("invitations.take-back") })
    .click();
  await expect(open.getByText(email)).toHaveCount(0);

  const guest = await anotherBrowser(browser, server);
  await guest.goto(link);
  await expect(guest.getByRole("alert")).toHaveText(t("error.auth.invitation-invalid"));
  await guest.context().close();
});

test("auth_06 a link without its secret says so", async ({ page }) => {
  await page.goto(paths.invitation);
  await expect(
    page.getByRole("heading", { level: 1, name: t("invitation.invalid-title") }),
  ).toBeVisible();
  await expect(page.getByText(t("invitation.incomplete"))).toBeVisible();
});

test("perf_02 a long list is shown a page at a time", async ({ page }) => {
  await inOrder(
    Array.from({ length: 22 }, (_, number) => async () => {
      await invite(admin, `many${String(number)}@example.test`);
    }),
  );
  await openInvitations(page);
  const items = page.getByRole("list", { name: t("invitations.open") }).getByRole("listitem");
  await expect(items).toHaveCount(20);
  await page.getByRole("button", { name: t("invitations.more") }).click();
  await expect(items.nth(21)).toBeVisible();
  await expect(page.getByRole("button", { name: t("invitations.more") })).toHaveCount(0);
});

test("auth_06 people who are not administrators do not see invitations", async ({ page }) => {
  await signIn(page, cleo);
  await openMenu(page);
  await expect(page.getByRole("link", { name: t("nav.organization-settings") })).toHaveCount(0);
  // The screen is not shown to them at all, as if it did not exist (M2-F17).
  await page.goto(paths.people);
  await expect(page.getByRole("heading", { name: t("page.not-found.title") })).toBeVisible();
});
