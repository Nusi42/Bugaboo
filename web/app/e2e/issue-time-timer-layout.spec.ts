import { t } from "@bugaboo/messages";

import { signedInAdmin } from "./support/access.ts";
import { ada } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { openIssueWithDetails, signIn } from "./support/flows.ts";
import { partIn } from "./support/parts.ts";
import {
  lengthNames,
  prepareTimeLayout,
  runTimerWithoutLengthsInTheLayout,
} from "./support/time-layout.ts";

// A timer that runs on an issue shows in its details wherever the layout puts the lengths of
// time, and when it places none of them (TIME-03, CFG-01).

test.use({ preparation: { run: prepareTimeLayout } });

test("time_03 a running timer shows in the details though the layout places no length of time", async ({
  page,
  server,
}) => {
  const admin = await signedInAdmin(server);
  await runTimerWithoutLengthsInTheLayout(admin);
  await admin.dispose();
  await signIn(page, ada);
  await openIssueWithDetails(page, "WEB-1");
  const rows = partIn(page, "name-value.name");
  await expect(rows.filter({ hasText: t("time.timer.row") })).toBeVisible();
  const lengths = new RegExp(Object.values(lengthNames).join("|"), "u");
  await expect(rows.filter({ hasText: lengths })).toHaveCount(0);
});
