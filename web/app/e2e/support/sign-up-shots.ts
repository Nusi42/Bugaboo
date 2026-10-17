import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { giveRole, roleName, signedInAdmin } from "./access.ts";
import { cleo, createPerson, ownId } from "./accounts.ts";
import { type Instance } from "./server.ts";
import { inEveryTheme } from "./shots-themes.ts";
import { paths } from "../../src/shared/paths.ts";

/** Creates a second administrator, and lets people of example.test sign up (e-mail is still off). */
export async function prepareShots(server: Instance): Promise<void> {
  const admin = await signedInAdmin(server);
  const second = await createPerson(server, admin, cleo);
  await giveRole(admin, await ownId(second), roleName.administrator, null);
  await second.dispose();
  const policy = dataOf(await admin.client.GET("/api/v1/auth/sign-up-policy"));
  await admin.client.PUT("/api/v1/auth/sign-up-policy", {
    body: { enabled: true, domains: ["example.test"], version: policy.version },
  });
  await admin.dispose();
}

/** Signing up: the form, and Check your e-mail. */
export async function shootSignUp(page: Page): Promise<void> {
  await page.goto(paths.signUp);
  await inEveryTheme(page, "20-sign-up");
  await page.getByLabel(t("field.email")).fill("newcomer@example.test");
  await page.getByRole("button", { name: t("sign-up.submit") }).click();
  await expect(
    page.getByRole("heading", { level: 1, name: t("sign-up.check-title") }),
  ).toBeVisible();
  await inEveryTheme(page, "21-check-your-e-mail");
}
