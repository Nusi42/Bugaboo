import { t } from "@bugaboo/messages";
import { type Browser, expect, type Page, type Response } from "@playwright/test";

import { type Person } from "./accounts.ts";
import { itemArea, layoutOf } from "./layout.ts";
import { type Instance } from "./server.ts";
import { type CodeApp, codeApp } from "./totp.ts";
import { issueAddress, paths } from "../../src/shared/paths.ts";
import { type ApiSession, sessionOver } from "./api.ts";
import { places } from "../../src/shared/places.ts";

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
  await page.goto(paths.signIn);
  await fillSignIn(page, person, password);
}

/** Types the setup code on the first step of the setup and goes on to the account. */
export async function enterSetupCode(page: Page, code: string): Promise<void> {
  await page.getByLabel(t("first-run.code.label")).fill(code);
  await page.getByRole("button", { name: t("common.continue") }).click();
  await expect(page.getByRole("heading", { name: t("first-run.account.title") })).toBeVisible();
}

/** Fills in the account of the second step of the setup and sends it. */
export async function sendSetupAccount(
  page: Page,
  person: Person,
  password = person.password,
): Promise<void> {
  await page.getByLabel(t("field.name")).fill(person.name);
  await page.getByLabel(t("field.email")).fill(person.email);
  await page.getByLabel(t("field.password"), { exact: true }).fill(password);
  await page.getByRole("button", { name: t("first-run.account.submit") }).click();
}

/** The fourth step of the first start, the mail server, which can be left for later: goes on. */
export async function skipMailStep(page: Page): Promise<void> {
  await expect(
    page.getByRole("heading", { level: 1, name: t("first-run.mail.title") }),
  ).toBeVisible();
  await page.getByRole("button", { name: t("common.continue"), exact: true }).click();
}

/** Types a code on the second step of signing in and sends it. */
export async function sendSecondStepCode(page: Page, code: string): Promise<void> {
  await page.getByLabel(t("second-step.code")).fill(code);
  await page.getByRole("button", { name: t("second-step.submit") }).click();
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

/**
 * Makes the app see the person with only these permissions for the whole organization, whatever
 * they hold (there are no screens yet to give a person a role). The server is not changed.
 */
export async function holdOnly(page: Page, permissions: readonly string[]): Promise<void> {
  await page.route("**/api/v1/auth/me", async (route) => {
    const answer = await route.fetch();
    // Nobody is signed in yet: the answer is the server's own.
    if (!answer.ok()) {
      await route.fulfill({ response: answer });
      return;
    }
    const body: unknown = await answer.json();
    if (typeof body !== "object" || body === null) {
      throw new Error("the answer about the person is not an object");
    }
    await route.fulfill({ response: answer, json: { ...body, permissions } });
  });
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

/** The topics of *Organization settings*, by the name people read; every other topic is in *My settings*. */
const organizationTopics: ReadonlySet<string> = new Set(
  places
    .filter((place) => place.parent === paths.organizationSettings)
    .map((place) => t(place.name)),
);

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

/** The API session of a browser that is signed in: it talks with the cookies of that browser. */
export function apiOf(page: Page, server: Instance): ApiSession {
  return sessionOver(page.context().request, server.url);
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

/** Where the person is once the backup codes are saved. */
export const afterCodeApp = {
  /** On the security page, where the backup codes are listed now. */
  securityPage: "security-page",
  /** On the screen that asks for a second step, which lists no codes. */
  secondStepScreen: "second-step-screen",
} as const;

/** What a test looks at on the way through the setup of the code app, and where it ends. */
export interface CodeAppSteps {
  /** Called when the picture to scan shows. */
  readonly onPicture?: () => Promise<void>;
  /** Called when the backup codes show, before they are saved. */
  readonly onBackupCodes?: () => Promise<void>;
  /** Where the person is once the codes are saved; the security page by default. */
  readonly ending?: (typeof afterCodeApp)[keyof typeof afterCodeApp];
}

/**
 * Sets up the code app, as a person does, and keeps the backup codes. On the security page the
 * backup codes are listed once the person has saved them; on the screen that asks for a second
 * step there is no such list (`ending: afterCodeApp.secondStepScreen`).
 */
export async function setUpCodeApp(
  page: Page,
  password: string,
  steps: CodeAppSteps = {},
): Promise<CodeAppSetUp> {
  const section = page.getByRole("region", { name: t("code-app.title") });
  await section.getByLabel(t("field.confirm-password"), { exact: true }).fill(password);
  await section.getByRole("button", { name: t("code-app.setup") }).click();
  await expect(page.getByRole("img", { name: t("code-app.qr-alt") })).toBeVisible();
  await steps.onPicture?.();
  const app = codeApp((await section.locator("code").textContent()) ?? "");
  await section.getByLabel(t("code-app.confirm-code")).fill(await app.next());
  await section.getByRole("button", { name: t("code-app.confirm") }).click();
  await expect(page.getByText(t("backup.shown-once"))).toBeVisible();
  const codes = page.getByRole("list", { name: t("backup.list") }).getByRole("listitem");
  const backupCodes = await codes.allTextContents();
  await steps.onBackupCodes?.();
  await page.getByRole("button", { name: t("backup.saved") }).click();
  if (steps.ending !== afterCodeApp.secondStepScreen) {
    await expect(page.getByRole("region", { name: t("backup.title") })).toBeVisible();
  }
  return { app, backupCodes };
}

/** Every address in the history of the browser tab, one text: what the back button could show. */
export async function historyAddresses(page: Page): Promise<string> {
  const tab = await page.context().newCDPSession(page);
  const { entries } = await tab.send("Page.getNavigationHistory");
  return entries.map((entry: { readonly url: string }) => entry.url).join(" ");
}

/** `text` as a pattern that matches it literally. */
function literally(text: string): string {
  return text.replaceAll(/[.*+?^${}()|[\]\\/]/gu, String.raw`\$&`);
}

/** The address bar ends with this address: the page is open, and no other below it. */
export function addressEndingAt(address: string): RegExp {
  return new RegExp(`${literally(address)}$`, "u");
}

/** The address bar holds this address, and may go on (a page below it, or a query). */
export function addressWith(address: string): RegExp {
  return new RegExp(literally(address), "u");
}

/** What this browser keeps in its storage, as `key=value` lines. */
export function storedText(page: Page): Promise<string> {
  return page.evaluate(() => {
    const lines: string[] = [];
    for (const store of [localStorage, sessionStorage]) {
      for (const key of Object.keys(store)) {
        lines.push(`${key}=${store.getItem(key) ?? ""}`);
      }
    }
    return lines.join(", ");
  });
}

/** Puts what the device kept before the choices lived on the server into its storage. */
export async function keepOnDevice(
  page: Page,
  entries: readonly (readonly [string, string])[],
): Promise<void> {
  await page.evaluate((kept: readonly (readonly [string, string])[]) => {
    for (const [key, value] of kept) {
      localStorage.setItem(key, value);
    }
  }, entries);
}

/** Gives *Go home* the keys `g i` on *Keyboard shortcuts*, so the list of keys has one of the person's own. */
export async function chooseGoHomeKeys(page: Page): Promise<void> {
  await page.goto(paths.shortcuts);
  await page.getByRole("button", { name: t("shortcut.go-home"), exact: true }).click();
  const panel = itemArea(page, t("shortcut.go-home"));
  await panel.getByRole("textbox", { name: t("shortcuts.new-keys"), exact: true }).click();
  await page.keyboard.press("g");
  await page.keyboard.press("i");
  await panel.getByRole("button", { name: t("shortcuts.use") }).click();
  await expect(panel.getByText(t("shortcuts.saved"))).toBeVisible();
}

/**
 * Opens an issue and, on a screen without room for a column of details, opens *All details*, where
 * the own fields are (UX §4.4).
 */
export async function openIssueWithDetails(page: Page, key: string): Promise<void> {
  await page.goto(issueAddress(key));
  if (layoutOf(page) !== "computer") {
    await page.getByRole("button", { name: t("issues.all-details"), exact: true }).click();
  }
}

/** Waits until the server has answered a request to follow (`PUT`) or to stop following (`DELETE`). */
export async function serverKeptFollow(page: Page, method: "PUT" | "DELETE"): Promise<void> {
  await page.waitForResponse(
    (response: Readonly<Response>) =>
      response.request().method() === method && response.url().includes("/follow") && response.ok(),
  );
}

export { itemArea };
