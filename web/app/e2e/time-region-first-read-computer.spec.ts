import { t } from "@bugaboo/messages";

import { paths } from "../src/shared/paths.ts";
import { bob } from "./support/accounts.ts";
import { expect, test } from "./support/fixtures.ts";
import { signIn } from "./support/flows.ts";
import { prepareWeb } from "./support/issues.ts";
import { cellOf, columnOf, today } from "./support/timesheet.ts";

// The timesheet writes its times in the person's Region (TIME-04, M8-F221): when the read of the person's settings
// before the first page had no answer, the page waits for the read again rather than writing the times in the device's
// way first and then again in the person's.

test.use({ preparation: { run: prepareWeb } });

/** The status of a server that is too busy to answer. */
const BUSY = 429;

/** The address of the person's settings. */
const SETTINGS = "**/api/v1/me/settings";

test("time_08 the timesheet waits for the person's region when its first read had no answer", async ({
  page,
}) => {
  await signIn(page, bob);
  let askedAgain = false;
  const answer = Promise.withResolvers<null>();
  // The read again waits for the test; the first read, before the first page, gets no answer (the newer route first).
  await page.route(SETTINGS, async (route) => {
    askedAgain = true;
    await answer.promise;
    await route.continue();
  });
  // A server too busy to answer, as under load (a refusal the app takes as no answer, not as a defect of its own).
  await page.route(SETTINGS, (route) => route.fulfill({ status: BUSY }), { times: 1 });
  await page.goto(paths.time);
  await expect.poll(() => askedAgain).toBe(true);
  // While the region is read again no time of the week is written: the week waits, with grey shapes.
  const start = cellOf(page, t("time.sheet.start"), columnOf(today()));
  await expect(start).toHaveCount(0);
  answer.resolve(null);
  await expect(start).toBeVisible();
});
