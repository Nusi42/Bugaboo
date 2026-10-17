import { t } from "@bugaboo/messages";

import { issueAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expectNoSidewaysScroll } from "./support/checks.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { prepareLook } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";
import { boxOf } from "./support/windows.ts";

// The top of the issue page on a phone (UX §3.3, M3 WP-17): the bar with Back and the steps, and
// the header with the type, the ID and More each stay on one line, so the header does not wrap.

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

test("ux_03 on a phone the header of an issue does not wrap: Back and the steps, the type and More", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "phone", "only a phone has this header");
  await signIn(page, ada);
  await page.goto(issueAddress("WEB-1"));
  await expect(page.getByRole("heading", { level: 1, name: /Login fails/u })).toBeVisible();
  const bar = ["focus.back", "list.previous", "list.next"] as const;
  const tops = await Promise.all(
    bar.map(
      async (key) => (await boxOf(page.getByRole("button", { name: t(key), exact: true }))).y,
    ),
  );
  expect(Math.max(...tops) - Math.min(...tops)).toBeLessThanOrEqual(2);
  const key = await boxOf(
    page.getByRole("button", { name: t("issues.copy-link-of", { key: "WEB-1" }), exact: true }),
  );
  const more = await boxOf(
    page.getByRole("button", { name: t("issues.more"), exact: true }).first(),
  );
  expect(Math.abs(key.y - more.y)).toBeLessThanOrEqual(2);
  // A phone has no second way to open the issue: it already fills the screen.
  await expect(
    page.getByRole("button", { name: t("issues.side-panel-view"), exact: true }),
  ).toHaveCount(0);
  await expectNoSidewaysScroll(page);
});
