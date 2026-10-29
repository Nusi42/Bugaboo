import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { ada, createAdmin } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { layoutOf } from "./support/layout.ts";
import { partIn } from "./support/parts.ts";
import { fillCreate } from "./support/projects.ts";
import { boxOf } from "./support/windows.ts";

// A row of the list of projects (UX §5.1; M5 exit A13): the name and the short name on one line,
// where the project sits muted under them, and a chevron that says the row opens.

test.use({
  preparation: {
    run: async (server) => {
      await (await createAdmin(server)).dispose();
    },
  },
});

test("ux_5_1 a project row has its name and short name on one line, the place under them and a chevron (A13)", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "the rows are the same on a phone; one size is enough");
  await signIn(page, ada);
  await page.goto(paths.projects);
  await fillCreate(page, "Website", "web");
  const row = page.getByRole("button", { name: "Website", exact: true });
  await expect(row).toBeVisible();
  const name = await boxOf(partIn(row, "list-editor.name"));
  const short = await boxOf(partIn(row, "projects.short"));
  const place = await boxOf(partIn(row, "projects.place"));
  expect(Math.abs(name.y + name.height / 2 - (short.y + short.height / 2))).toBeLessThanOrEqual(4);
  expect(place.y).toBeGreaterThanOrEqual(name.y + name.height - 1);
  await expect(partIn(row, "list-editor.chevron")).toBeVisible();
  await expect(partIn(row, "projects.place")).toContainText(t("projects.at-the-top"));
});
