import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { ada, cleo, makeAdmin, makePerson } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { holdOnly, openFromMenu, openMenu, signIn } from "./support/flows.ts";
import { onThisLayout } from "./support/layout.ts";

test.use({
  preparation: {
    run: async (server) => {
      const admin = await makeAdmin(server);
      await (await makePerson(server, admin, cleo)).dispose();
      await admin.dispose();
    },
  },
});

/** The menu of the app: the sidebar, or what a tablet slides in. */
function mainMenu(page: Page): Locator {
  return page.getByRole("navigation", { name: t("shell.menu"), exact: true });
}

/** Screens that come with later versions (New with M3, Notifications with M4): no entry yet. */
const later = /^(New|Notifications)$/u;

/** Nothing in the menu is for a screen that does not exist yet. */
async function expectNoLaterScreens(menu: Locator): Promise<void> {
  await expect(menu.getByRole("link", { name: later })).toHaveCount(0);
  await expect(menu.getByRole("button", { name: later })).toHaveCount(0);
}

async function computerFrame(page: Page): Promise<void> {
  const menu = mainMenu(page);
  await expect(page.getByRole("button", { name: t("shell.menu-button"), exact: true })).toHaveCount(
    0,
  );
  await expect(menu.getByRole("link", { name: t("nav.home"), exact: true })).toBeVisible();
  await expect(menu.getByRole("link", { name: t("nav.my-settings"), exact: true })).toBeVisible();
  await expect(menu.getByRole("link", { name: t("nav.organization-settings") })).toBeVisible();
  await expect(menu.getByRole("button", { name: t("shortcut.title") })).toBeVisible();
  await expectNoLaterScreens(menu);
  // The sidebar narrows to icons and the words stay for screen readers.
  await menu.getByRole("button", { name: t("shell.narrow") }).click();
  await expect(menu.getByRole("link", { name: t("nav.home"), exact: true })).toBeVisible();
  await expect(menu.getByText(t("shell.signed-in-as", { name: ada.name }))).toHaveCount(0);
  await menu.getByRole("button", { name: t("shell.widen") }).click();
  await expect(menu.getByText(t("shell.signed-in-as", { name: ada.name }))).toBeVisible();
}

async function tabletFrame(page: Page): Promise<void> {
  await expect(mainMenu(page)).toHaveCount(0);
  const button = page.getByRole("button", { name: t("shell.menu-button"), exact: true });
  await button.click();
  const menu = mainMenu(page);
  await expect(menu.getByRole("link", { name: t("nav.home"), exact: true })).toBeVisible();
  await expect(menu.getByRole("link", { name: t("nav.organization-settings") })).toBeVisible();
  await expectNoLaterScreens(menu);
  // Esc closes the menu and the focus goes back to the button that opened it.
  await page.keyboard.press("Escape");
  await expect(mainMenu(page)).toHaveCount(0);
  await expect(button).toBeFocused();
  // A link of the menu goes there and closes the menu.
  await button.click();
  await mainMenu(page)
    .getByRole("link", { name: t("nav.my-settings"), exact: true })
    .click();
  await expect(page).toHaveURL(/\/me\/appearance$/u);
  await expect(mainMenu(page)).toHaveCount(0);
}

async function phoneFrame(page: Page): Promise<void> {
  const bar = mainMenu(page);
  // The bar holds only screens that exist: Home, Search and More.
  await expect(bar.getByRole("link")).toHaveCount(1);
  await expect(bar.getByRole("button")).toHaveCount(2);
  await expect(bar.getByRole("link", { name: t("nav.home"), exact: true })).toBeVisible();
  await expect(bar.getByRole("button", { name: t("shell.search"), exact: true })).toBeVisible();
  await expectNoLaterScreens(bar);
  await bar.getByRole("button", { name: t("shell.more"), exact: true }).click();
  const sheet = page.getByRole("dialog", { name: t("shell.more"), exact: true });
  await expect(sheet.getByRole("link", { name: t("nav.my-settings"), exact: true })).toBeVisible();
  await expect(
    sheet.getByRole("link", { name: t("nav.organization-settings"), exact: true }),
  ).toBeVisible();
  await expect(sheet.getByRole("button", { name: t("nav.sign-out"), exact: true })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(sheet).toHaveCount(0);
}

test("ux_01 the frame of each size shows the screens that exist and nothing else", async ({
  page,
}) => {
  await signIn(page, ada);
  await onThisLayout(page, { computer: computerFrame, tablet: tabletFrame, phone: phoneFrame });
  await expectGoodInEveryLook(page);
});

async function trailOnLargeScreens(page: Page): Promise<void> {
  const trail = page.getByRole("navigation", { name: t("shell.trail") });
  await expect(trail.getByRole("link", { name: t("nav.my-settings") })).toBeVisible();
  await expect(trail.getByText(t("nav.security"))).toHaveAttribute("aria-current", "page");
  await trail.getByRole("link", { name: t("nav.my-settings") }).click();
  await expect(page).toHaveURL(/\/me\/appearance$/u);
}

async function backOnPhone(page: Page): Promise<void> {
  await expect(page.getByRole("button", { name: t("shell.back"), exact: true })).toBeVisible();
  // Back goes to where the person came from: the list of topics.
  await page.getByRole("button", { name: t("shell.back"), exact: true }).click();
  await expect(page).toHaveURL(/\/me$/u);
  await expect(page.getByRole("heading", { level: 1, name: t("nav.my-settings") })).toBeVisible();
  // The phone's own back gesture does the same with the history.
  await page.getByRole("link", { name: t("nav.security"), exact: true }).click();
  await expect(page).toHaveURL(/\/me\/security$/u);
  await page.goBack();
  await expect(page).toHaveURL(/\/me$/u);
  // Home has nothing to go back to: no Back button.
  await page.goto("/");
  await expect(page.getByRole("button", { name: t("shell.back"), exact: true })).toHaveCount(0);
}

test("ux_01 the top bar says where you are, and a phone has Back", async ({ page }) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.security"));
  await expect(page.getByRole("heading", { level: 1, name: t("security.title") })).toBeVisible();
  await onThisLayout(page, {
    computer: trailOnLargeScreens,
    tablet: trailOnLargeScreens,
    phone: backOnPhone,
  });
});

/** The links to the topics of a settings frame, wherever the frame shows them. */
function topics(page: Page, heading: string): Locator {
  return page.getByRole("navigation", { name: heading, exact: true }).getByRole("link");
}

async function topicsOnLargeScreens(page: Page): Promise<void> {
  await page
    .getByRole("link", { name: t("nav.my-settings"), exact: true })
    .first()
    .click();
  await expect(page).toHaveURL(/\/me\/appearance$/u);
  await expect(topics(page, t("nav.my-settings"))).toHaveText([
    t("nav.appearance"),
    t("shortcut.title"),
    t("nav.security"),
    t("nav.sessions"),
    t("nav.access-keys"),
  ]);
}

async function topicsOnPhone(page: Page): Promise<void> {
  await openMenu(page);
  await page
    .getByRole("dialog")
    .getByRole("link", { name: t("nav.my-settings") })
    .click();
  // One screen at a time: the topics are the page of the settings, and a topic is a page of its own.
  await expect(page).toHaveURL(/\/me$/u);
  await expect(page.getByRole("heading", { level: 1, name: t("nav.my-settings") })).toBeVisible();
  await expect(topics(page, t("nav.my-settings"))).toHaveText([
    t("nav.appearance"),
    t("shortcut.title"),
    t("nav.security"),
    t("nav.sessions"),
    t("nav.access-keys"),
  ]);
}

test("ux_01 My settings lists its topics and nothing for administrators only", async ({ page }) => {
  await signIn(page, cleo);
  await onThisLayout(page, {
    computer: topicsOnLargeScreens,
    tablet: async (shown) => {
      await openMenu(shown);
      await topicsOnLargeScreens(shown);
    },
    phone: topicsOnPhone,
  });
  await openMenu(page);
  await expect(page.getByRole("link", { name: t("nav.organization-settings") })).toHaveCount(0);
  await expectGoodInEveryLook(page);
});

test("ux_01 Organization settings lists the topics that exist, by heading", async ({ page }) => {
  await signIn(page, ada);
  await openFromMenu(page, t("nav.invitations"));
  await expect(page.getByRole("heading", { level: 1, name: t("invitations.title") })).toBeVisible();
  // A phone shows one screen at a time: the topics are on the page that Back goes to.
  await onThisLayout(page, {
    computer: async () => {},
    tablet: async () => {},
    phone: async (shown) => {
      await shown.getByRole("button", { name: t("shell.back"), exact: true }).click();
    },
  });
  await expect(topics(page, t("nav.organization-settings"))).toHaveText([
    t("nav.invitations"),
    t("nav.status"),
  ]);
  await topics(page, t("nav.organization-settings")).last().click();
  await expect(page).toHaveURL(/\/settings\/status$/u);
  await expectGoodInEveryLook(page);
});

test("ux_01 an address of the settings that does not exist shows the page that is not there", async ({
  page,
}) => {
  await signIn(page, cleo);
  // A person who may not invite people finds nothing at the address, like at one that is not there.
  await page.goto("/settings/invitations");
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeVisible();
  await page.goto("/me/nothing-here");
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeVisible();
});

test("sec_01 the start page offers looking after the server only to people who may open organization settings", async ({
  page,
}) => {
  // A Viewer holds a permission (see projects), but none that opens organization settings.
  await holdOnly(page, ["see-projects"]);
  await signIn(page, cleo);
  await expect(
    page.getByRole("heading", { level: 1, name: t("home.title", { name: cleo.name }) }),
  ).toBeVisible();
  await expect(page.getByText(t("home.intro"))).toBeVisible();
  await expect(page.getByText(t("home.admin"))).toHaveCount(0);
});

test("ux_01 the tab says the name of the topic once, also when a topic is opened by its address", async ({
  page,
}) => {
  await signIn(page, ada);
  await page.goto("/me/appearance");
  await expect(page).toHaveTitle(`${t("appearance.title")} · ${t("app.name")}`);
  await page.goto("/me/shortcuts");
  await expect(page).toHaveTitle(`${t("shortcut.title")} · ${t("app.name")}`);
  await page.goto("/me/security");
  await expect(page).toHaveTitle(`${t("security.title")} · ${t("app.name")}`);
});
