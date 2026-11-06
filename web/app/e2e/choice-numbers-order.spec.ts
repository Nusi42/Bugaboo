import { t } from "@bugaboo/messages";

import { fieldAddress } from "../src/shared/paths.ts";
import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { addCostCenters } from "./support/cost-centers.ts";
import { expect, test } from "./support/fixtures.ts";
import { fieldOf } from "./support/fields.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { choiceNamed } from "./support/time.ts";

// Only the choices without a number are put in order by hand: a number places the others (TIME-13).

test.use({ preparation: { run: prepareWeb } });

test("time_13 only the choices without a number are put in order by hand", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  const field = await fieldOf(admin, "Cost center");
  await addCostCenters(admin);
  await choiceNamed(admin, field.id, "Miscellaneous");
  await signIn(page, ada);
  await page.goto(fieldAddress(field.id));
  const list = page.getByRole("list", { name: t("fields.choices"), exact: true });
  const items = list.getByRole("listitem");
  // A number places a choice: no arrows and no handle for the three that have one.
  const numbered = items.filter({ has: page.locator("[data-part='choice-number']") });
  await expect(numbered).toHaveCount(3);
  await expect(numbered.getByRole("button", { name: /^Move /u })).toHaveCount(0);
  // Among the two without a number the first cannot go up and the last cannot go down.
  const up = (name: string) =>
    list.getByRole("button", { name: t("fields.choice-up", { name }), exact: true });
  const down = (name: string) =>
    list.getByRole("button", { name: t("fields.choice-down", { name }), exact: true });
  await expect(up("Overhead")).toBeDisabled();
  await expect(down("Miscellaneous")).toBeDisabled();
  // Moving one is a change that shows at once.
  await down("Overhead").click();
  await expect(items.nth(3)).toContainText("Miscellaneous");
  await expect(items.nth(4)).toContainText("Overhead");
  await admin.dispose();
});
