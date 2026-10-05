import { t } from "@bugaboo/messages";
import { type Browser, expect, type Page } from "@playwright/test";

import { type Person } from "./accounts.ts";
import { layoutOf } from "./layout.ts";
import { type Instance } from "./server.ts";
import { type CodeApp, codeApp } from "./totp.ts";

/** Types the e-mail address and the password into the sign-in form that is open, and sends them. */
export async function fillSignIn(
  page: Page,
  person: Person,
  password = person.password,
): Promise<void> {
  await page.getByLabel(t("field.email")).fill(person.email);
  await page.getByLabel(t("field.password"), { exact: true }).fill(password);
  await page.getByRole("button", { name: t("sign-in.submit"), exact: true }).click();
}

/** Opens the sign-in page, types the e-mail address and the password, and sends them. */
export async function sendPassword(
  page: Page,
  person: Person,
  password = person.password,
): Promise<void> {
  await page.goto("/sign-in");
  await fillSignIn(page, person, password);
}

/** The start page of the person is open. */
export async function expectHome(page: Page, person: Person): Promise<void> {
  await expect(
    page.getByRole("heading", { level: 1, name: t("home.title", { name: person.name }) }),
  ).toBeVisible();
}

/** Signs in with a password, for a person who has no second step. */
export async function signIn(page: Page, person: Person): Promise<void> {
  await sendPassword(page, person);
  await expectHome(page, person);
}

/** Opens what holds the menu when the screen has no sidebar: *Menu* on a tablet, *More* on a phone. */
export async function openMenu(page: Page): Promise<void> {
  const layout = layoutOf(page);
  if (layout !== "computer") {
    const name = layout === "tablet" ? t("shell.menu-button") : t("shell.more");
    await page.getByRole("button", { name, exact: true }).click();
  }
}

/** Presses *Sign out*, wherever the menu keeps it on this screen. */
export async function pressSignOut(page: Page): Promise<void> {
  await openMenu(page);
  await page.getByRole("button", { name: t("nav.sign-out"), exact: true }).click();
}

/** Signs out with the button of the menu, and waits for the sign-in page. */
export async function signOut(page: Page): Promise<void> {
  await pressSignOut(page);
  await expect(page.getByRole("heading", { level: 1, name: t("sign-in.title") })).toBeVisible();
}

/** The topics of *Organization settings*; every other topic is in *My settings*. */
const organizationTopics: ReadonlySet<string> = new Set([t("nav.invitations"), t("nav.status")]);

/**
 * Opens a topic of *My settings* or *Organization settings* the way a person does: the menu (a
 * sidebar, the *Menu* button or the *More* sheet), the settings, then the topic in the frame.
 */
export async function openFromMenu(page: Page, topic: string): Promise<void> {
  const settings = organizationTopics.has(topic)
    ? t("nav.organization-settings")
    : t("nav.my-settings");
  await openMenu(page);
  // On a phone the links are in the sheet that *More* opens, on larger screens in the menu.
  const menu =
    layoutOf(page) === "phone"
      ? page.getByRole("dialog", { name: t("shell.more"), exact: true })
      : page.getByRole("navigation", { name: t("shell.menu"), exact: true });
  await menu.getByRole("link", { name: settings, exact: true }).click();
  await page
    .getByRole("navigation", { name: settings, exact: true })
    .getByRole("link", { name: topic, exact: true })
    .click();
}

/** Opens another browser of its own, with its own cookies, as a second person or a second device. */
export async function anotherBrowser(browser: Browser, server: Instance): Promise<Page> {
  const context = await browser.newContext({ baseURL: server.url, ignoreHTTPSErrors: true });
  return context.newPage();
}

/** Adds a passkey on the security page, after the password was typed again. */
export async function addPasskey(page: Page, label: string, password: string): Promise<void> {
  const passkeys = page.getByRole("region", { name: t("passkeys.title") });
  await passkeys.getByLabel(t("passkeys.name")).fill(label);
  await passkeys.getByLabel(t("field.confirm-password"), { exact: true }).fill(password);
  await passkeys.getByRole("button", { name: t("passkeys.add") }).click();
  await expect(page.getByText(t("passkeys.added", { name: label }))).toBeVisible();
}

/** A code app that was set up on the security page, and the backup codes that were shown. */
export interface CodeAppSetUp {
  readonly app: CodeApp;
  readonly backupCodes: readonly string[];
}

/** Sets up the code app on the security page, as a person does, and keeps the backup codes. */
export async function setUpCodeApp(page: Page, password: string): Promise<CodeAppSetUp> {
  const section = page.getByRole("region", { name: t("code-app.title") });
  await section.getByLabel(t("field.confirm-password"), { exact: true }).fill(password);
  await section.getByRole("button", { name: t("code-app.setup") }).click();
  await expect(page.getByRole("img", { name: t("code-app.qr-alt") })).toBeVisible();
  const app = codeApp((await section.locator("code").textContent()) ?? "");
  await section.getByLabel(t("code-app.confirm-code")).fill(await app.next());
  await section.getByRole("button", { name: t("code-app.confirm") }).click();
  await expect(page.getByText(t("backup.shown-once"))).toBeVisible();
  const codes = page.getByRole("list", { name: t("backup.list") }).getByRole("listitem");
  const backupCodes = await codes.allTextContents();
  await page.getByRole("button", { name: t("backup.saved") }).click();
  await expect(page.getByRole("region", { name: t("backup.title") })).toBeVisible();
  return { app, backupCodes };
}

/** Every address in the history of the browser tab, one text: what the back button could show. */
export async function historyAddresses(page: Page): Promise<string> {
  const tab = await page.context().newCDPSession(page);
  const { entries } = await tab.send("Page.getNavigationHistory");
  return entries.map((entry: { readonly url: string }) => entry.url).join(" ");
}
