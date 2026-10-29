import { projectIssuesAddress } from "../src/shared/paths.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { prepareLook, rowOf } from "./support/list-look.ts";
import { partIn } from "./support/parts.ts";

// The columns that fit (UX §4.2; M5 exit L12): a width a person gave a column counts, so a column
// dragged wide does not push the list sideways; the columns that do not fit are dropped.

test.use({ preparation: { run: prepareLook } });

test("ux_4_2 a column dragged wide at 1280 does not make the list scroll sideways", async ({
  page,
}) => {
  test.skip(layoutOf(page) !== "computer", "only a computer has columns to drag");
  await signIn(page, ada);
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto(
    `${projectIssuesAddress("WEB")}?cols=status,id,title,priority,assignedTo,changed&widths=assignedTo:600`,
  );
  await expect(rowOf(page, webIssues.login)).toBeVisible();
  const frame = partIn(page, "list-frame").first();
  expect(await frame.evaluate((box) => box.scrollWidth - box.clientWidth)).toBe(0);
});
