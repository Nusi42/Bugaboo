import { addDays } from "../../src/shared/format.ts";
import { signedInAdmin } from "./access.ts";
import { createIssue } from "./issues.ts";
import type { Instance } from "./server.ts";
import { choiceNamed, starterField } from "./time.ts";
import { bookOn, today, weekOf } from "./timesheet.ts";

/**
 * The first day of this week, where the administrator has booked time on one issue with two
 * different cost centers on two days: the row of the timesheet then names both.
 */
export async function weekWithCostCenters(server: Instance): Promise<string> {
  const admin = await signedInAdmin(server);
  const issue = await createIssue(admin, "WEB", "Printer jams");
  const activity = await starterField(admin, "activity");
  const costCenter = await starterField(admin, "cost-center");
  const first = weekOf(today());
  const testing = await choiceNamed(admin, activity, "Testing");
  const product = await choiceNamed(admin, costCenter, "Product development", "4100");
  const quality = await choiceNamed(admin, costCenter, "Quality assurance", "0420");
  await bookOn(admin, issue, first, 90, { [activity]: testing, [costCenter]: product });
  await bookOn(admin, issue, addDays(first, 1), 60, { [activity]: testing, [costCenter]: quality });
  await admin.dispose();
  return first;
}
