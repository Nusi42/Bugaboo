import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { chooseTheme, expectNoSidewaysScroll } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { prepareLook } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";
import {
  backgroundOf,
  daylight,
  expectEqualParts,
  expectFullWidth,
  shadowAtEndOf,
  spread,
  topsOf,
} from "./support/phone-issue.ts";

// The issue page on a phone (UX §3.3, §4.4, M5 exit L1): the top bar of every page with Back and the
// issue ID, Previous and Next in the page, and the bar of actions under the page.

test.use({ preparation: { run: prepareLook } });

test("ux_03 on a phone the closed Details button says who has the issue and its priority with its icon (M3-F184)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has this button");
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const toggle = partIn(page, "issue-details-toggle");
  // Nobody has the issue: the summary says so in words, not by leaving a bare word.
  await expect(toggle.getByText(t("issues.summary-unassigned"))).toBeVisible();
  // The priority with its icon, as everywhere else.
  await expect(partIn(toggle, "priority")).toContainText(t("priority.critical"));
  await expect(partIn(toggle, "priority-icon")).toBeVisible();
  // The way to the first person says what it does.
  await page.getByRole("button", { name: t("issues.all-details"), exact: true }).click();
  await expect(
    page.getByRole("button", { name: t("issues.assign-someone"), exact: true }),
  ).toBeVisible();
});

test("ux_3_3 on a phone the issue page has the top bar of every page, Previous and Next in the page and one More (L1)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has this bar");
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await expect(page.getByRole("heading", { level: 1, name: /Login fails/u })).toBeVisible();
  // The shared bar: Back and the ID of the issue, as on every other screen of a phone.
  const bar = page.getByRole("banner");
  await expect(bar.getByRole("button", { name: t("shell.back"), exact: true })).toBeVisible();
  await expect(partIn(bar, "top-bar.title")).toHaveText("WEB-1");
  await expect(page.getByRole("button", { name: t("focus.back"), exact: true })).toHaveCount(0);
  // Previous and Next stay in the page, on one line.
  expect(spread(await topsOf(page, ["list.previous", "list.next"]))).toBeLessThanOrEqual(1);
  // *More* of the issue once: the one of the bar of actions, none in the page (the bar of the app has its own).
  await expect(
    page.getByRole("main").getByRole("button", { name: t("issues.more"), exact: true }),
  ).toHaveCount(0);
  await expect(
    page
      .getByRole("toolbar", { name: t("issues.bar"), exact: true })
      .getByRole("button", { name: t("issues.more"), exact: true }),
  ).toBeVisible();
  // A phone has no second way to open the issue: it already fills the screen.
  await expect(
    page.getByRole("button", { name: t("issues.side-panel-view"), exact: true }),
  ).toHaveCount(0);
  await expectNoSidewaysScroll(page);
});

test("ux_3_3 on a phone the bar of actions fills its width in equal parts, stands apart from the page and casts a shadow over what scrolls under it", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has this bar");
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await expect(page.getByRole("heading", { level: 1, name: /Login fails/u })).toBeVisible();
  await chooseTheme(page, daylight());
  const bar = page.getByRole("toolbar", { name: t("issues.bar"), exact: true });
  await expectEqualParts(bar);
  // A surface of its own: not the color of the page in Daylight.
  expect(await backgroundOf(partIn(page, "issue-bar"))).not.toBe(
    await backgroundOf(partIn(page, "app-layout")),
  );
  // The page scrolls under the bar: a shadow at its end while there is more below, none at the end.
  const main = partIn(page, "app-layout.main");
  await expect.poll(() => shadowAtEndOf(main)).toBe(1);
  await main.evaluate((node) => {
    node.scrollTop = node.scrollHeight;
  });
  await expect.poll(() => shadowAtEndOf(main)).toBe(0);
  // The page has no shadow at its top: it scrolls under the top bar (L6).
  expect(await main.evaluate((node) => getComputedStyle(node, "::before").content)).toBe("none");
});

test("ux_3_3 on a phone the button that sends a comment is as wide as the screen, on a row of its own", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has this rule");
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  const box = page.getByRole("textbox", { name: t("comments.write"), exact: true });
  const form = page.locator("form").filter({ has: box });
  await expectFullWidth(form.getByRole("button", { name: t("comments.add"), exact: true }), form);
});
