import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { grid, rowOf } from "./support/list-look.ts";
import { preparePanel } from "./support/list-panel.ts";
import { partIn } from "./support/parts.ts";
import { boxOf } from "./support/windows.ts";

// The list beside the side panel has the room that is left (UX §4.2, M5 exit finding L1/A2): it
// keeps the status, the ID, the title and the priority, nothing of it runs under the panel and
// nothing scrolls sideways.

test.use({ preparation: { run: preparePanel } });

test("ux_4_2 beside the open issue the list ends 16 px before the panel and does not scroll sideways (L1)", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "computer", "only a computer has the panel beside the list");
  await signIn(page, ada);
  await page.goto(`${projectIssuesAddress("WEB")}?open=WEB-1`);
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  const panel = await boxOf(page.getByRole("complementary").first());
  const frame = partIn(page, "list-frame").first();
  const list = await boxOf(frame);
  expect(list.x + list.width).toBeLessThanOrEqual(panel.x - 16 + 1);
  const scrolls = await frame.evaluate((element) => element.scrollWidth - element.clientWidth);
  expect(scrolls).toBeLessThanOrEqual(0);
  // The row of the open issue is ringed whole, inside the list.
  const row = await boxOf(rowOf(page, webIssues.login));
  expect(row.x + row.width).toBeLessThanOrEqual(list.x + list.width + 1);
  // The title keeps room to be read.
  const title = await boxOf(grid(page).getByRole("link", { name: webIssues.login }));
  expect(title.width).toBeGreaterThanOrEqual(100);
});
