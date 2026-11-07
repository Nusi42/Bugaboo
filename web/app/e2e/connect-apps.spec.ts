import { t } from "@bugaboo/messages";
import { type Page } from "@playwright/test";

import { ada, createAdmin } from "./support/accounts.ts";
import { expectGoodInEveryTheme } from "./support/checks.ts";
import { appAtTheDoor, catchTheAnswer, codeOf, switchAiOn } from "./support/connect-app.ts";
import { expect, test } from "./support/fixtures.ts";
import { fillSignIn, openFromMenu, signIn } from "./support/flows.ts";

// An AI app asks to work with the account: the consent step and *Connected apps* (AI-22, AI-24,
// ADR-0042, M7-Q3), on a computer and on a phone.

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
      await switchAiOn(server);
    },
  },
});
test.describe.configure({ mode: "serial" });

/** The word of the heading of the consent step for an app. */
function titleOf(app: string): string {
  return t("connect.title", { app });
}

/** The tokens of an app work at `/mcp`: the answer to a ping is `200`, and `401` once they are taken back. */
async function pingStatus(page: Page, address: string, access: string): Promise<number> {
  const answered = await page.request.post(`${address}/mcp`, {
    headers: { authorization: `Bearer ${access}` },
    data: { jsonrpc: "2.0", id: 1, method: "ping" },
  });
  return answered.status();
}

test("ai_22 the person sees which app asks, allows it, and the app gets its code", async ({
  page,
  server,
}) => {
  const app = await appAtTheDoor(server, "Claude Code", "see-and-change");
  await signIn(page, ada);
  await page.goto(app.address);
  await expect(page.getByRole("heading", { level: 1, name: titleOf("Claude Code") })).toBeVisible();
  await expect(page.getByText(t("connect.acts-as", { app: "Claude Code" }))).toBeVisible();
  await expect(page.getByText(t("connect.where-computer"))).toBeVisible();
  // The app asked for more, so both choices are offered, with what it asked first.
  await expect(
    page.getByRole("radio", { name: t("access-keys.choice.see-and-change") }),
  ).toBeChecked();
  await expectGoodInEveryTheme(page);
  const answer = await catchTheAnswer(page, async () => {
    await page.getByRole("button", { name: t("connect.allow"), exact: true }).click();
  });
  expect(answer.searchParams.get("state")).toBe("state-1");
  const tokens = await app.exchange(codeOf(answer.href));
  expect(await pingStatus(page, server.url, tokens.access)).toBe(200);
});

test("ai_22 taking an app back ends what it may do at once", async ({ page, server }) => {
  const app = await appAtTheDoor(server, "Cursor", "see-and-change");
  await signIn(page, ada);
  await page.goto(app.address);
  const answer = await catchTheAnswer(page, async () => {
    await page.getByRole("button", { name: t("connect.allow"), exact: true }).click();
  });
  const tokens = await app.exchange(codeOf(answer.href));
  // The app is in the list, and taking it back ends it at once.
  await page.goto("/");
  await openFromMenu(page, t("nav.connected-apps"));
  await expect(
    page.getByRole("heading", { level: 1, name: t("connected-apps.title") }),
  ).toBeVisible();
  const row = page.getByRole("row").filter({ hasText: "Cursor" });
  await expect(row).toBeVisible();
  await expect(row.getByText(t("access-keys.choice.see-and-change"))).toBeVisible();
  await expectGoodInEveryTheme(page);
  await row
    .getByRole("button", {
      name: t("connected-apps.take-back-name", { name: "Cursor" }),
    })
    .click();
  await page
    .getByRole("alertdialog")
    .getByRole("button", {
      name: t("connected-apps.take-back-confirm"),
      exact: true,
    })
    .click();
  await expect(row).toHaveCount(0);
  expect(await pingStatus(page, server.url, tokens.access)).toBe(401);
});

test("ai_22 the person may allow less than the app asked, and cancel sends the app away", async ({
  page,
  server,
}) => {
  const app = await appAtTheDoor(server, "Claude Desktop", "see-and-change");
  await signIn(page, ada);
  await page.goto(app.address);
  await page.getByRole("radio", { name: t("access-keys.choice.see-only") }).check();
  const answer = await catchTheAnswer(page, async () => {
    await page.getByRole("button", { name: t("connect.allow"), exact: true }).click();
  });
  const tokens = await app.exchange(codeOf(answer.href));
  const listed = await page.request.post(`${server.url}/mcp`, {
    headers: { authorization: `Bearer ${tokens.access}` },
    data: { jsonrpc: "2.0", id: 1, method: "tools/list" },
  });
  const names = JSON.stringify(await listed.json());
  expect(names).toContain("search_issues");
  expect(names).not.toContain("create_issue");
  // Cancel: the app is sent back with the refusal, and nothing is allowed.
  const another = await appAtTheDoor(server, "Claude Code", "see-only");
  await page.goto(another.address);
  await expect(page.getByText(t("connect.may-only"))).toBeVisible();
  const refused = await catchTheAnswer(page, async () => {
    await page.getByRole("button", { name: t("connect.cancel"), exact: true }).click();
  });
  expect(refused.searchParams.get("error")).toBe("access_denied");
  expect(refused.searchParams.get("code")).toBeNull();
});

test("ai_22 a person who is not signed in signs in first and comes back to the consent step", async ({
  page,
  server,
}) => {
  const app = await appAtTheDoor(server, "Claude Code", "see-only");
  await page.goto(app.address);
  await fillSignIn(page, ada);
  await expect(page.getByRole("heading", { level: 1, name: titleOf("Claude Code") })).toBeVisible();
});

test("ai_22 a request from an app nobody registered is told in plain words and allows nothing", async ({
  page,
  server,
}) => {
  const app = await appAtTheDoor(server, "Claude Code", "see-only");
  await signIn(page, ada);
  await page.goto(app.address.replace(app.client, "5b0f6a3e-2d1c-4e8a-9f77-0a1b2c3d4e5f"));
  await expect(page.getByText(t("connect.unknown"))).toBeVisible();
  await expect(page.getByRole("button", { name: t("connect.allow"), exact: true })).toHaveCount(0);
});
