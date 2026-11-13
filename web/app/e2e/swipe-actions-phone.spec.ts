import { t } from "@bugaboo/messages";

import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { moreOf, swipe, swipedActions, swipeStrip } from "./support/swipe.ts";
import {
  adminSession,
  followsIssue,
  putStatusBack,
  refuseStopFollowing,
  setFollowing,
  statusNow,
} from "./support/swipe-checks.ts";
import { firstRow } from "./support/swipe-lists.ts";
import { listText } from "../src/shared/list-text.ts";
import { projectIssuesAddress } from "../src/shared/paths.ts";

// What the swipe actions of a row of the issue list do (UX-02, UX §3.3): they change the issue on the server and on
// the screen, the focus goes where a person expects it, and from right to left the sides follow the writing.

test.use({ sample: true });

/** The first row of the issue list of WEB, and the ID of its issue. */
async function firstIssueRow(page: Parameters<typeof firstRow>[0]) {
  const row = await firstRow(page, projectIssuesAddress("WEB"), "list-row");
  const key = (await partIn(row, "issue-id").first().innerText()).trim();
  return { row: partIn(page, "list-row").filter({ hasText: key }), key };
}

test("ux_02 Follow on a swipe follows the issue on the server, and stops again", async ({
  page,
  server,
}) => {
  await signIn(page, sampleAdministrator());
  const { row, key } = await firstIssueRow(page);
  const admin = await adminSession(server);
  const before = await followsIssue(admin, key);
  await setFollowing(admin, key, false);
  await page.reload();
  // A finger towards the end brings in the start: *Follow*, as the server says the person does not follow.
  await swipe(page, row, "end");
  await expect(swipedActions(row).first()).toHaveText(t("followers.follow"));
  await swipedActions(row).first().tap();
  await expect.poll(() => followsIssue(admin, key)).toBe(true);
  await swipe(page, row, "end");
  await expect(swipedActions(row).first()).toHaveText(t("followers.stop"));
  await swipedActions(row).first().tap();
  await expect.poll(() => followsIssue(admin, key)).toBe(false);
  await setFollowing(admin, key, before);
  await admin.dispose();
});

test("ux_02 Change status or priority on a swipe changes that one issue on the screen and on the server", async ({
  page,
  server,
}) => {
  await signIn(page, sampleAdministrator());
  const { row, key } = await firstIssueRow(page);
  const admin = await adminSession(server);
  const before = await statusNow(admin, key);
  await swipe(page, row, "start");
  await swipedActions(row).getByText(t("issues.row-change-status"), { exact: true }).tap();
  const dialog = page.getByRole("alertdialog", {
    name: t("many-changes.title", { action: t("many-changes.action-change"), count: 1 }),
  });
  await dialog.getByLabel(t("issue-field.status"), { exact: true }).selectOption({ index: 1 });
  await dialog.getByRole("button", { name: t("many-changes.show-changes"), exact: true }).tap();
  await dialog
    .getByRole("button", { name: t("many-changes.apply", { count: 1 }), exact: true })
    .tap();
  await expect(dialog).toBeHidden();
  await expect.poll(async () => (await statusNow(admin, key)).key).not.toBe(before.key);
  const after = await statusNow(admin, key);
  await expect(row.getByRole("img", { name: after.name, exact: true })).toBeVisible();
  await putStatusBack(admin, key, before.key);
  await admin.dispose();
});

test("ux_02 after a swiped action the focus is on More, Escape and a press outside close the side, and opening is announced", async ({
  page,
  server,
}) => {
  await signIn(page, sampleAdministrator());
  const { row, key } = await firstIssueRow(page);
  const admin = await adminSession(server);
  const before = await followsIssue(admin, key);
  await swipe(page, row, "start");
  const names = await swipedActions(row).allInnerTexts();
  await expect(row.getByRole("status")).toHaveText(
    t("common.swipe-shown", { actions: listText(names) }),
  );
  await page.keyboard.press("Escape");
  await expect(swipeStrip(row)).toHaveCount(0);
  await swipe(page, row, "start");
  await page.locator("[data-part='top-bar.title']").tap();
  await expect(swipeStrip(row)).toHaveCount(0);
  // After an action the focus is on the row's *More*, where the person can go on.
  await swipe(page, row, "end");
  await swipedActions(row).first().tap();
  await expect(moreOf(row)).toBeFocused();
  await setFollowing(admin, key, before);
  await admin.dispose();
});

test("ux_02 from right to left a swipe opens the side of the writing and it slides in from that edge", async ({
  page,
}) => {
  await signIn(page, sampleAdministrator());
  const { row } = await firstIssueRow(page);
  await page.evaluate(() => {
    document.documentElement.dir = "rtl";
  });
  // Towards the left is towards the end when the page is written from right to left: it brings in the start.
  await swipe(page, row, "start");
  const strip = swipeStrip(row);
  await expect(strip).toHaveAttribute("data-variant", "start");
  expect(await strip.evaluate((node) => getComputedStyle(node).animationName)).toBe(
    "in-from-start",
  );
});

test("ux_02 a Stop following that the server refuses says so on the row", async ({
  page,
  server,
}) => {
  await signIn(page, sampleAdministrator());
  const { row, key } = await firstIssueRow(page);
  const admin = await adminSession(server);
  const before = await followsIssue(admin, key);
  await setFollowing(admin, key, true);
  await page.reload();
  await refuseStopFollowing(page);
  await swipe(page, row, "end");
  await swipedActions(row).getByText(t("followers.stop"), { exact: true }).tap();
  await expect(row.getByRole("alert")).toHaveText(t("error.access.forbidden"));
  await setFollowing(admin, key, before);
  await admin.dispose();
});
