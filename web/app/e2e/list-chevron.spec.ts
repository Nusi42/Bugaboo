import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";
import { sampleAdministrator } from "./support/sample.ts";
import { inOrder } from "./support/sequence.ts";
import { paths } from "../src/shared/paths.ts";

// M5 exit, package O (O16): a row of a list that opens something has the sign that it does: a chevron at its end.

test.use({ sample: true });

test("ux_5 the rows of the lists that open an item have a chevron", async ({ page }) => {
  await signIn(page, sampleAdministrator());
  await inOrder(
    [paths.groups, paths.labels, paths.issueTypes, paths.relationKinds, paths.projects].map(
      (address) => async () => {
        await page.goto(address);
        const row = partIn(page, "list-editor.open").first();
        await expect(row).toBeVisible();
        await expect(row.locator("svg")).toHaveCount(1);
      },
    ),
  );
});
