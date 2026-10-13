import { t } from "@bugaboo/messages";
import { type Locator, type Page } from "@playwright/test";

import { ada, cleo, makeAdmin, makePerson } from "./support/accounts.ts";
import { expectGoodInEveryLook } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { addressEndingAt, holdOnly, openFromMenu, openMenu, signIn } from "./support/flows.ts";
import { onThisLayout } from "./support/layout.ts";
import { paths } from "../src/shared/paths.ts";

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
  await expect(page).toHaveURL(addressEndingAt(paths.appearance));
  await expect(mainMenu(page)).toHaveCount(0);
}

async function phoneFrame(page: Page): Promise<void> {
  const bar = mainMenu(page);
  // The bar holds only screens that exist: Home, Search, New and More.
  await expect(bar.getByRole("link")).toHaveCount(1);
  await expect(bar.getByRole("button")).toHaveCount(3);
  await expect(bar.getByRole("button", { name: t("issues.new"), exact: true })).toBeVisible();
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
  await expect(page).toHaveURL(addressEndingAt(paths.appearance));
}

async function backOnPhone(page: Page): Promise<void> {
  await expect(page.getByRole("button", { name: t("shell.back"), exact: true })).toBeVisible();
  // Back goes to where the person came from: the list of topics.
  await page.getByRole("button", { name: t("shell.back"), exact: true }).click();
  await expect(page).toHaveURL(addressEndingAt(paths.mySettings));
  await expect(page.getByRole("heading", { level: 1, name: t("nav.my-settings") })).toBeVisible();
  // The phone's own back gesture does the same with the history.
  await page.getByRole("link", { name: t("nav.security"), exact: true }).click();
  await expect(page).toHaveURL(addressEndingAt(paths.security));
  await page.goBack();
  await expect(page).toHaveURL(addressEndingAt(paths.mySettings));
  // Home has nothing to go back to: no Back button.
  await page.goto(paths.home);
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
  await expect(page).toHaveURL(addressEndingAt(paths.appearance));
  await expect(topics(page, t("nav.my-settings"))).toHaveText([
    t("nav.appearance"),
    t("nav.region"),
    t("nav.start-page"),
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
  await expect(page).toHaveURL(addressEndingAt(paths.mySettings));
  await expect(page.getByRole("heading", { level: 1, name: t("nav.my-settings") })).toBeVisible();
  await expect(topics(page, t("nav.my-settings"))).toHaveText([
    t("nav.appearance"),
    t("nav.region"),
    t("nav.start-page"),
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
  await openFromMenu(page, t("nav.people"));
  await expect(page.getByRole("heading", { level: 1, name: t("nav.people") })).toBeVisible();
  // A phone shows one screen at a time: the topics are on the page that Back goes to.
  await onThisLayout(page, {
    computer: async () => {},
    tablet: async () => {},
    phone: async (shown) => {
      await shown.getByRole("button", { name: t("shell.back"), exact: true }).click();
    },
  });
  await expect(topics(page, t("nav.organization-settings"))).toHaveText([
    t("nav.people"),
    t("nav.groups"),
    t("nav.roles"),
    t("nav.sign-in-rules"),
    t("nav.field-permissions"),
    t("nav.people-visibility"),
    t("nav.check-access"),
    t("nav.themes"),
    t("nav.projects"),
    t("nav.issue-types"),
    t("nav.fields"),
    t("nav.relation-kinds"),
    t("nav.organization-security"),
    t("nav.status"),
  ]);
  await topics(page, t("nav.organization-settings")).last().click();
  await expect(page).toHaveURL(addressEndingAt(paths.status));
  await expectGoodInEveryLook(page);
});

test("ux_01 an address of the settings that does not exist shows the page that is not there", async ({
  page,
}) => {
  await signIn(page, cleo);
  // A person who may not invite people finds nothing at the address, like at one that is not there.
  await page.goto(paths.people);
  await expect(
    page.getByRole("heading", { level: 1, name: t("page.not-found.title") }),
  ).toBeVisible();
  await page.goto(`${paths.mySettings}/nothing-here`);
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
  await page.goto(paths.appearance);
  await expect(page).toHaveTitle(`${t("appearance.title")} · ${t("app.name")}`);
  await page.goto(paths.shortcuts);
  await expect(page).toHaveTitle(`${t("shortcut.title")} · ${t("app.name")}`);
  await page.goto(paths.security);
  await expect(page).toHaveTitle(`${t("security.title")} · ${t("app.name")}`);
});

test("ux_01 a page that loads when it is opened gets the focus on its heading", async ({
  page,
}) => {
  await signIn(page, ada);
  // The Themes page is a chunk of its own: the focus must wait for it and not land on the page
  // that is leaving.
  await openFromMenu(page, t("nav.themes"));
  await expect(page).toHaveURL(addressEndingAt(paths.themes));
  await expect(page.getByRole("heading", { level: 1, name: t("nav.themes") })).toBeFocused();
});
