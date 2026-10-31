import { dataOf, settingKeys } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Browser, type Locator, type Page } from "@playwright/test";

import { type ApiSession } from "./api.ts";
import { assignTo, issueOf, readIssue, versionOf } from "./issues.ts";
import { bob } from "./accounts.ts";
import { anotherBrowser, signIn } from "./flows.ts";
import { type Instance } from "./server.ts";
import { issueAddress, paths } from "../../src/shared/paths.ts";

/**
 * The entry *Notifications* of the menu: a link of the sidebar on a computer, of the menu a tablet
 * slides in, and of the bottom bar of a phone. It says its number after its name.
 */
export function menuEntry(page: Page): Locator {
  return page
    .getByRole("navigation", { name: t("shell.menu"), exact: true })
    .getByRole("link", { name: new RegExp(`^${t("nav.notifications")}`, "u") });
}

/** The number on the menu entry; 0 when it shows none. */
export async function unreadCount(page: Page): Promise<number> {
  const text = (await menuEntry(page).textContent()) ?? "";
  return Number(/\d+/u.exec(text)?.[0] ?? 0);
}

/**
 * Does what a person does that makes the app ask again at once: come back to the tab. The app also
 * asks every half minute, which a test does not wait for.
 */
export async function comeBackToTheTab(page: Page): Promise<void> {
  await page.evaluate(() => {
    window.dispatchEvent(new Event("visibilitychange"));
  });
}

/** Waits until the menu shows this number, coming back to the tab to make the app ask. */
export async function expectUnread(page: Page, count: number): Promise<void> {
  await expect
    .poll(async () => {
      await comeBackToTheTab(page);
      return unreadCount(page);
    })
    .toBe(count);
}

/** The handle of the person of the session, without the @. */
export async function handleOf(session: ApiSession): Promise<string> {
  return dataOf(await session.client.GET("/api/v1/me/handle")).handle;
}

/** Writes a comment under an issue, as the person of the session. */
export async function comment(session: ApiSession, issue: string, text: string): Promise<string> {
  const made = dataOf(
    await session.client.POST("/api/v1/issues/{issue}/comments", {
      params: { path: { issue } },
      body: { text },
    }),
  );
  return made.id;
}

/** Tells the person of the session in the app of every change of a field on issues they follow. */
export async function hearOfFieldChanges(session: ApiSession): Promise<void> {
  await session.client.PUT("/api/v1/me/settings/{key}", {
    params: { path: { key: settingKeys.notificationsFollowedFieldsInApp } },
    body: { value: true },
  });
}

/** Makes the person of the session follow an issue. */
export async function follow(session: ApiSession, issue: string): Promise<void> {
  await session.client.PUT("/api/v1/issues/{issue}/follow", { params: { path: { issue } } });
}

/** Opens the page of notifications. */
export async function openNotifications(page: Page): Promise<void> {
  await page.goto(paths.notifications);
  await expect(
    page.getByRole("heading", { level: 1, name: t("notifications.title"), exact: true }),
  ).toBeVisible();
}

/** Ben (Bob Builder) in a browser of his own, signed in. */
export async function asBen(browser: Browser, server: Instance): Promise<Page> {
  const ben = await anotherBrowser(browser, server);
  await signIn(ben, bob);
  return ben;
}

/** Writes a comment with the box on the page of the issue, the way a person does. */
export async function writeCommentOnThePage(
  page: Page,
  issue: string,
  text: string,
): Promise<void> {
  await page.goto(issueAddress(issue));
  const box = page.getByRole("textbox", { name: t("comments.write"), exact: true });
  await box.fill(text);
  await page
    .locator("form")
    .filter({ has: box })
    .getByRole("button", { name: t("comments.add"), exact: true })
    .click();
  await expect(page.getByText(text)).toBeVisible();
}

/** Changes the status of an issue, as the person of the session. */
export async function changeStatus(
  session: ApiSession,
  issue: string,
  status: string,
): Promise<void> {
  const { version } = (await readIssue(session, { key: issue })).issue;
  await session.client.PATCH("/api/v1/issues/{issue}", {
    params: { path: { issue } },
    body: { version: version ?? 1, status },
  });
}

/** Marks every notification of the person of the session as read, so a test starts from none. */
export async function readEverything(session: ApiSession): Promise<void> {
  const newest = dataOf(
    await session.client.GET("/api/v1/notifications", { params: { query: { limit: 1 } } }),
  ).items[0]?.id;
  if (newest !== undefined) {
    await session.client.POST("/api/v1/notifications/read", { body: { up_to: newest } });
  }
}

/** The ID of the person of the session. */
async function ownIdOf(session: ApiSession): Promise<string> {
  return dataOf(await session.client.GET("/api/v1/auth/me")).id;
}

/** Ben does things on the issues Ana follows: a comment, a status change, a mention, an assignment. */
export async function benActs(ben: ApiSession, anaSession: ApiSession): Promise<void> {
  await comment(ben, "WEB-1", "I could reproduce it.\n\n- Reset the password\n- Sign in again");
  await changeStatus(ben, "WEB-1", "in-progress");
  const handle = await handleOf(anaSession);
  await comment(ben, "WEB-2", `Ana, can you look at the **passkey** flow? @${handle}`);
  const issue = await issueOf(ben, "WEB-2");
  await assignTo(ben, issue, await ownIdOf(anaSession), await versionOf(ben, issue));
}
