import { t } from "@bugaboo/messages";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { prepareLook, rowOf } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";
import { boxOf } from "./support/windows.ts";

// Group headers and an empty list (UX §4.2; M5 exit L20, L25): the header stands on the columns of
// the rows, and a list with nothing in it has no head and says so in the middle.

test.use({ preparation: { run: prepareLook } });

test("ux_4_2 a group header stands on the columns of the rows: its icon on the status icon, its name on the ID (L20)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "computer", "only a computer has columns");
  await signIn(page, ada);
  await page.goto(`${projectIssuesAddress("WEB")}?group=statusGroup`);
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  const kind = await boxOf(partIn(page, "list-group.kind").first());
  const name = await boxOf(partIn(page, "list-group.name").first());
  const status = await boxOf(partIn(page, "list-row.cell", "status").first());
  const key = await boxOf(partIn(page, "list-row.cell", "key").first());
  expect(Math.abs(kind.x + kind.width / 2 - (status.x + status.width / 2))).toBeLessThanOrEqual(2);
  expect(Math.abs(name.x - key.x)).toBeLessThanOrEqual(2);
});

test("ux_4_2 a list with nothing in it has no head to tick and says so in the middle (L25)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "computer", "only a computer has a head");
  await signIn(page, ada);
  await page.goto(`${projectIssuesAddress("WEB")}?s=title+contains+zzzzqq`);
  const message = page.getByText(t("search.found-none"), { exact: true });
  await expect(message).toBeVisible();
  await expect(
    page.getByRole("checkbox", { name: t("issues.select-shown"), exact: true }),
  ).toHaveCount(0);
  const text = await boxOf(message);
  const list = await boxOf(partIn(page, "list-frame").first());
  expect(Math.abs(text.x + text.width / 2 - (list.x + list.width / 2))).toBeLessThanOrEqual(2);
});
