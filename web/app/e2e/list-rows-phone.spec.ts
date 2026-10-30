import { expect } from "@playwright/test";

import { test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";
import { boxOf } from "./support/windows.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O: a row of a list on a phone that has only its name (a group, an issue type with its icon before the
// name, a kind of relation) is one line, and the line is in the middle of the row.

test.use({ sample: true });

test("ux_5 a row with only its name is one line in the middle of the row", async ({ page }) => {
  test.skip(layoutOf(page) !== "phone", "the rows of a computer have their own look");
  await signIn(page, sampleAdministrator());
  await inOrder(
    [paths.groups, paths.issueTypes, paths.relationKinds].map((address) => async () => {
      await page.goto(address);
      const row = await boxOf(partIn(page, "list-editor.open").first());
      const name = await boxOf(partIn(page, "list-editor.name").first());
      const middle = Math.abs(name.y + name.height / 2 - (row.y + row.height / 2));
      expect(
        middle,
        `${address}: the name is level with the middle of its row`,
      ).toBeLessThanOrEqual(2);
    }),
  );
});
