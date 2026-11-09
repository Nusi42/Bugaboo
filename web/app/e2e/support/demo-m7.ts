import { t } from "@bugaboo/messages";
import { expect, type Page } from "@playwright/test";

import { paths } from "../../src/shared/paths.ts";
import { appAtTheDoor, catchTheAnswer, codeOf } from "./connect-app.ts";
import { adaOfTheSample, anaOfTheSample, good } from "./demo-m7-common.ts";
import { openIssueWithDetails, signIn, signOut } from "./flows.ts";
import { type Instance } from "./server.ts";
import { exchange } from "./session-transport.ts";

// The first two steps of the exit demo of M7 on the sample organization: an AI app of a Contributor
// and the assistant (in `demo-m7-assistant.ts`). The agent steps are in `demo-m7-agents.ts`.

const ISSUE = "WEB-142";

/** The own field the sample hides from Contributors, on the page and from an AI app. */
const HIDDEN_FROM_CONTRIBUTORS = "Cost center";

/** An own field both people see, near the two that are hidden from Contributors. */
const LAST_OWN_FIELD = "Followers";

/** The text an app is given for a tool of `/mcp`, with its token. */
async function toolText(server: Instance, token: string, name: string, args: object) {
  const answered = await exchange(
    {
      url: `${server.url}/mcp`,
      method: "POST",
      headers: {
        "content-type": "application/json",
        accept: "application/json, text/event-stream",
        authorization: `Bearer ${token}`,
      },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "tools/call",
        params: { name, arguments: args },
      }),
    },
    server.url,
  );
  return answered.body;
}

/** Step 1: an MCP client connects as a Contributor and does not see what is hidden from Contributors. */
export async function aContributorsAppSeesLessThanTheTeam(page: Page, server: Instance) {
  const door = await appAtTheDoor(server, "Claude Code", "see-only");
  await signIn(page, anaOfTheSample());
  await page.goto(door.address);
  // On a phone the buttons are under the page; the picture shows them.
  await page
    .getByRole("button", { name: t("connect.cancel"), exact: true })
    .scrollIntoViewIfNeeded();
  await good(page, "01-a-the-app-asks");
  const answer = await catchTheAnswer(page, async () => {
    await page.getByRole("button", { name: t("connect.allow"), exact: true }).click();
  });
  const tokens = await door.exchange(codeOf(answer.toString()));
  const seen = await toolText(server, tokens.access, "get_issue", { issue: ISSUE });
  expect(seen).toContain(ISSUE);
  // The sample marks Customer and Cost center Never sent to AI and hides them from Contributors, so an
  // app that works as a Contributor is not given them for two reasons.
  expect(seen).not.toContain("Customer");
  expect(seen).not.toContain(HIDDEN_FROM_CONTRIBUTORS);
  // The page of the app that is back is the app's own; the list of apps shows what was allowed.
  await page.goto(paths.connectedApps);
  await expect(page.getByText("Claude Code").first()).toBeVisible();
  await good(page, "01-b-the-app-is-allowed");
  // What the app was not shown is not on Ana's page either; Ada, an Administrator, sees both fields.
  await openIssueWithDetails(page, ISSUE);
  await expect(page.getByText(t("issue-field.priority"), { exact: true }).first()).toBeVisible();
  await expect(page.getByText(HIDDEN_FROM_CONTRIBUTORS)).toHaveCount(0);
  // The picture ends at the last of the details, where Ada's page has the two fields.
  await page.getByText(LAST_OWN_FIELD, { exact: true }).first().scrollIntoViewIfNeeded();
  await good(page, "01-c-what-ana-sees");
  // The bar of an open issue has its own *More*: the menu with *Sign out* is on the home page.
  await page.goto(paths.home);
  await signOut(page);
  await signIn(page, adaOfTheSample());
  await openIssueWithDetails(page, ISSUE);
  const hidden = page.getByText(HIDDEN_FROM_CONTRIBUTORS).first();
  // Customer, the other hidden field, is in the middle of the screen; Cost center is near it on a computer.
  await page
    .getByText("Customer", { exact: true })
    .first()
    .evaluate((label: HTMLElement) => {
      label.scrollIntoView({ block: "center" });
    });
  await expect(hidden).toBeVisible();
  await good(page, "01-d-what-ada-sees");
}
