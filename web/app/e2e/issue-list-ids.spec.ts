import { settingKeys } from "@bugaboo/api-client";

import { projectIssuesAddress } from "../src/shared/paths.ts";
import { createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { countIssueIds, createIssue } from "./support/issues.ts";
import { createProject } from "./support/projects.ts";
import {
  idsCut,
  keyWidth,
  openEveryList,
  useLargerText,
  widerCodeFont,
} from "./support/list-ids.ts";
import { grid } from "./support/list-look.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";

// The ID column of the list is as wide as the longest ID the list draws (UX §4.2): a short name of
// ten letters with a random code of eight characters is never cut, and a list of short IDs leaves
// the room to the other columns.

/** The longest short name a project may have, and the longest random code (the setting's bound). */
const LONG_SHORT_NAME = "ABCDEFGHIJ";
const LONGEST_CODE = 8;

test.use({
  preparation: {
    run: async (server) => {
      const admin = await createAdmin(server);
      await admin.client.PUT("/api/v1/settings/{key}", {
        params: { path: { key: settingKeys.issuesIdLength } },
        body: { value: LONGEST_CODE },
      });
      // Made while the IDs are random: a code of eight characters after the ten letters.
      await createProject(admin, LONG_SHORT_NAME, "Long names");
      await createIssue(admin, LONG_SHORT_NAME, "An issue with a long ID");
      await countIssueIds(admin);
      await createProject(admin, "AB", "Short names");
      await createIssue(admin, "AB", "An issue with a short ID");
      await admin.dispose();
    },
  },
});

test("ux_4_2 an ID of ten letters and eight characters is not cut", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "a card sizes its ID from what it says");
  await openEveryList(page);
  expect(await idsCut(page)).toBe(0);
});

test("ux_4_2 the same at the larger text", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "a card sizes its ID from what it says");
  await openEveryList(page);
  await useLargerText(page);
  expect(await idsCut(page)).toBe(0);
});

test("ux_4_2 a wider code font does not cut an ID: the character is measured, not assumed", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "a card sizes its ID from what it says");
  await openEveryList(page);
  const before = await keyWidth(grid(page));
  // A code font wider than any the theme file thought of: every character 4 px wider, in the
  // IDs and in the probe the column is measured with.
  await widerCodeFont(page);
  await expect.poll(() => keyWidth(grid(page))).toBeGreaterThan(before + 19 * 3);
  expect(await idsCut(page)).toBe(0);
});

test("ux_4_2 a list of short IDs leaves the room to the other columns", async ({ page }) => {
  test.skip(layoutOf(page) === "phone", "a card sizes its ID from what it says");
  await openEveryList(page);
  const mixed = await keyWidth(grid(page));
  await page.goto(projectIssuesAddress("AB"));
  await expect(partIn(grid(page), "list-row.cell", "key").first()).toBeVisible();
  expect(await keyWidth(grid(page))).toBeLessThan(mixed);
});
