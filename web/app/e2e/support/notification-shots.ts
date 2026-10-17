import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { ana } from "./access.ts";
import { bob, signInThroughApi } from "./accounts.ts";
import { signIn } from "./flows.ts";
import { benActs, follow, openNotifications } from "./notifications.ts";
import { type Instance } from "./server.ts";
import { inEveryTheme } from "./shots-themes.ts";
import { paths } from "../../src/shared/paths.ts";

/** Notifications of Ana: the start page, the page with its filters, Customize and the handle. */
export async function shootNotifications(page: Page, server: Instance): Promise<void> {
  const ben = await signInThroughApi(server, bob);
  const anaSession = await signInThroughApi(server, ana);
  await signIn(page, ana);
  await follow(anaSession, "WEB-1");
  await follow(anaSession, "WEB-2");
  await benActs(ben, anaSession);
  await page.goto(paths.home);
  await expect(
    page.getByRole("region", { name: t("notifications.title"), exact: true }),
  ).toBeVisible();
  await inEveryTheme(page, "01-start-page");
  await openNotifications(page);
  await expect(page.getByText(t("notifications.mentioned", { who: bob.name }))).toBeVisible();
  await inEveryTheme(page, "02-notifications");
  await page
    .getByRole("button", { name: new RegExp(`^${t("notifications.mark-read")}`, "u") })
    .last()
    .click();
  await expect(page.getByText(t("notifications.new"), { exact: true })).toHaveCount(3);
  await inEveryTheme(page, "03-notifications-one-read");
  await page.getByRole("tab", { name: t("notifications.filter.mentions"), exact: true }).click();
  await inEveryTheme(page, "04-notifications-mentions");
  await page.goto(paths.notificationSettings);
  await expect(
    page.getByRole("heading", { level: 1, name: t("notifications.customize-title") }),
  ).toBeVisible();
  await inEveryTheme(page, "05-customize");
  await page.goto(paths.handle);
  await page.getByLabel(t("handle.label"), { exact: true }).fill("bob.builder");
  await page.getByRole("button", { name: t("handle.save"), exact: true }).click();
  await expect(page.getByRole("alert")).toHaveText(t("handle.taken"));
  await inEveryTheme(page, "06-handle-refused");
}
