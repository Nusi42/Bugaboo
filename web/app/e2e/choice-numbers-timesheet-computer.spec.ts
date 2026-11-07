import { ada } from "./support/accounts.ts";
import { weekWithCostCenters } from "./support/cost-center-week.ts";
import { expect, test } from "./support/fixtures.ts";
import { topOf, topsOf } from "./support/geometry.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { openWeek } from "./support/timesheet.ts";

// The timesheet writes a cost center as its number in the code font and its name (TIME-13, M6-F143).

test.use({ preparation: { run: prepareWeb } });

test("time_13 the timesheet writes the cost centers of a row in full, with the number in the code font", async ({
  page,
  server,
}) => {
  const week = await weekWithCostCenters(server);
  await signIn(page, ada);
  await openWeek(page, week);
  const cell = page.getByRole("cell", {
    name: /^0420 Quality assurance.*4100 Product development/u,
  });
  await expect(cell).toBeVisible();
  await expect(cell.locator("[data-part='choice-number']")).toHaveText(["0420", "4100"]);
  // One choice on each line, in the order of the numbers, and every cell of the row starts at the top.
  const lines = cell.locator("[data-part='choice-line']");
  // A row with two cost centers says the length of each one after it (M6-F201).
  await expect(lines).toHaveText([
    /^0420 Quality assurance · \S+/u,
    /^4100 Product development · \S+/u,
  ]);
  expect(await topOf(lines.nth(1))).toBeGreaterThan(await topOf(lines.nth(0)));
  const issue = page.getByRole("row", { name: /Printer jams/u });
  const tops = await topsOf(issue.getByRole("cell"));
  expect(new Set(tops).size).toBe(1);
  // Written in full: nothing of it is cut off by the width of the column.
  const cut = await cell.evaluate((each) => each.scrollWidth > each.clientWidth);
  expect(cut).toBe(false);
});
